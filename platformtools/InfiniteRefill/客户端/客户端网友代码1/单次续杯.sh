#!/usr/bin/env bash
set -euo pipefail

# 单次续杯（探测 -> 上报状态 -> 触发 topup -> 写入新账号 -> 删除失效账号）
#
# 依赖：curl、python3
#
# 服务端契约：
# - POST /v1/refill/topup
#   Header: X-User-Key: <USER_KEY>
#   Body:
#     {"target_pool_size":10,"reports":[{"file_name":"x.json","email_hash":"...","account_id":"...","status_code":401,"probed_at":"2026-..Z"}]}
#   Resp:
#     {"ok":true,"accounts":[{"file_name":"无限续杯-001.json","download_url":"https://..."}], ...}

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

need_cmd() {
  local cmd="$1"
  command -v "$cmd" >/dev/null 2>&1 || {
    echo "[ERROR] 缺少依赖命令：$cmd"
    exit 127
  }
}

need_json_parser() {
  need_cmd python3
}

json_auth_fields4() {
  local f="$1"
  python3 - "$f" <<'PY'
import json, sys

path = sys.argv[1]
try:
    with open(path, 'r', encoding='utf-8') as fp:
        obj = json.load(fp)
except Exception:
    print('')
    print('')
    print('')
    print('')
    raise SystemExit(0)

auth = obj.get('auth') if isinstance(obj.get('auth'), dict) else {}

def pick(*vals):
  for val in vals:
    if isinstance(val, str) and val:
      return val
  return ''

print(pick(auth.get('type'), obj.get('type')))
print(pick(auth.get('access_token'), obj.get('access_token')))
print(pick(auth.get('account_id'), obj.get('account_id')))
print(pick(obj.get('email'), auth.get('email')))
PY
}

json_normalize_wham_status() {
  local raw_status="$1"
  local body_file="$2"

  if [[ "$raw_status" =~ ^[0-9]{3}$ ]] && [[ "$raw_status" != "000" ]]; then
    echo "$raw_status"
    return 0
  fi

  if [[ -f "$body_file" ]] && grep -Eqi 'quota|rate[[:space:]-_]*limit|too many|insufficient' "$body_file"; then
    echo "429"
    return 0
  fi

  echo "000"
}

json_topup_write_accounts_from_response() {
  local resp_json="$1"
  local accounts_dir="$2"
  local rows_file
  rows_file="${TMPDIR:-/tmp}/topup_rows_$$.tsv"

  decode_base64_to_file() {
    local data="$1"
    local out_file="$2"

    if printf '%s' "$data" | base64 --decode > "$out_file" 2>/dev/null; then
      return 0
    fi
    if printf '%s' "$data" | base64 -D > "$out_file" 2>/dev/null; then
      return 0
    fi
    return 1
  }

  python3 - "$resp_json" "$rows_file" <<'PY'
import base64
import json
import sys

resp_path = sys.argv[1]
rows_path = sys.argv[2]

with open(resp_path, 'r', encoding='utf-8') as fp:
    data = json.load(fp)

if data.get('ok') is False:
    raise SystemExit(2)

accounts = data.get('accounts')
if not isinstance(accounts, list):
    raise SystemExit(3)

with open(rows_path, 'w', encoding='utf-8') as out:
    for i, acc in enumerate(accounts, start=1):
        if not isinstance(acc, dict):
            continue
        file_name = acc.get('file_name') or f'无限续杯-{i:03d}.json'
        download_url = acc.get('download_url') or ''
        payload = acc.get('account_json')
        if payload is None:
            payload = acc.get('json')
        if payload is None:
            payload = acc.get('content')

        content_b64 = ''
        if isinstance(payload, dict):
            raw = json.dumps(payload, ensure_ascii=False).encode('utf-8')
            content_b64 = base64.b64encode(raw).decode('ascii')
        elif isinstance(payload, str) and payload.strip():
            content_b64 = base64.b64encode(payload.encode('utf-8')).decode('ascii')

        out.write(f"{file_name}\t{download_url}\t{content_b64}\n")
PY

  mkdir -p "$accounts_dir"
  local count=0
  local file_name download_url content_b64 out_path
  while IFS=$'\t' read -r file_name download_url content_b64; do
    [[ -n "$file_name" ]] || continue
    out_path="$accounts_dir/$file_name"

    if [[ -n "$content_b64" ]]; then
      if ! decode_base64_to_file "$content_b64" "$out_path"; then
        rm -f "$out_path" 2>/dev/null || true
        continue
      fi
      count=$((count+1))
      continue
    fi

    if [[ -n "$download_url" ]]; then
      if curl -fsSL "$download_url" -o "$out_path"; then
        count=$((count+1))
      else
        rm -f "$out_path" 2>/dev/null || true
      fi
    fi
  done < "$rows_file"

  rm -f "$rows_file" 2>/dev/null || true
  echo "$count"
}

need_cmd curl
need_json_parser

CFG_ENV="$SCRIPT_DIR/无限续杯配置.env"

MODE_SYNC_ALL=0
if [[ "${1:-}" == "--sync-all" ]]; then
  MODE_SYNC_ALL=1
  shift
fi

SERVER_URL="${1:-}"
USER_KEY="${2:-}"
ACCOUNTS_DIR="$SCRIPT_DIR/accounts"
TARGET_POOL_SIZE=10
TOTAL_HOLD_LIMIT=50
SYNC_TARGET_DIR=
WHAM_CONNECT_TIMEOUT=5
WHAM_MAX_TIME=15
TOPUP_CONNECT_TIMEOUT=8
TOPUP_MAX_TIME=30

if [[ -f "$CFG_ENV" ]]; then
  # shellcheck disable=SC1090
  source "$CFG_ENV" || true
fi

SERVER_URL="${SERVER_URL:-${SERVER_URL:-}}"
USER_KEY="${USER_KEY:-${USER_KEY:-}}"
ACCOUNTS_DIR="${ACCOUNTS_DIR:-$SCRIPT_DIR/accounts}"
TARGET_POOL_SIZE="${TARGET_POOL_SIZE:-10}"
TOTAL_HOLD_LIMIT="${TOTAL_HOLD_LIMIT:-50}"
SYNC_TARGET_DIR="${SYNC_TARGET_DIR:-}"
WHAM_CONNECT_TIMEOUT="${WHAM_CONNECT_TIMEOUT:-5}"
WHAM_MAX_TIME="${WHAM_MAX_TIME:-15}"
TOPUP_CONNECT_TIMEOUT="${TOPUP_CONNECT_TIMEOUT:-8}"
TOPUP_MAX_TIME="${TOPUP_MAX_TIME:-30}"

if [[ -z "$SERVER_URL" || -z "$USER_KEY" ]]; then
  echo "[ERROR] 未配置 SERVER_URL/USER_KEY，请先运行：bash \"$SCRIPT_DIR/无限续杯.sh\""
  exit 2
fi

mkdir -p "$ACCOUNTS_DIR"
if [[ ! -d "$ACCOUNTS_DIR" ]]; then
  echo "[ERROR] 账户目录不存在且创建失败：$ACCOUNTS_DIR"
  exit 3
fi

TS="$(date -u +%Y%m%d-%H%M%S)"
OUT_DIR="$SCRIPT_DIR/out/单次续杯-$TS"
REPORT_JSONL="$OUT_DIR/reports.jsonl"
BODY_JSON="$OUT_DIR/topup_body.json"
RESP_JSON="$OUT_DIR/topup_response.json"
BACKUP_DIR="$OUT_DIR/backup"
NETFAIL_LOG="$OUT_DIR/probe_netfail.log"

mkdir -p "$BACKUP_DIR"
: > "$REPORT_JSONL"
: > "$NETFAIL_LOG"

sync_managed_json() {
  local target
  target="$SYNC_TARGET_DIR"

  [[ -z "$target" ]] && return 0
  mkdir -p "$target"

  shopt -s nullglob
  local files=("$ACCOUNTS_DIR"/无限续杯-*.json)
  if [[ ${#files[@]} -eq 0 ]]; then
    files=("$ACCOUNTS_DIR"/*.json)
  fi

  local manifest="$target/.infinite_refill_sync_manifest.txt"
  local names=() f base tp

  for f in "${files[@]}"; do
    [[ -e "$f" ]] || continue
    base="$(basename "$f")"
    names+=("$base")
  done

  if [[ -f "$manifest" ]]; then
    while IFS= read -r base; do
      [[ -n "$base" ]] || continue
      tp="$target/$base"
      if [[ ! -e "$ACCOUNTS_DIR/$base" && -L "$tp" ]]; then
        rm -f "$tp" 2>/dev/null || true
      fi
    done < "$manifest"
  fi

  for f in "${files[@]}"; do
    [[ -e "$f" ]] || continue
    base="$(basename "$f")"
    tp="$target/$base"
    if [[ -L "$tp" ]]; then
      rm -f "$tp" 2>/dev/null || true
    elif [[ -e "$tp" ]]; then
      continue
    fi
    ln -s "$f" "$tp" 2>/dev/null || true
  done

  printf "%s\n" "${names[@]}" > "$manifest"
}

if [[ "$MODE_SYNC_ALL" == "1" ]]; then
  TS_SYNCALL="$(date -u +%Y%m%d-%H%M%S)"
  OUT_DIR="${TMPDIR:-/tmp}/InfiniteRefill-syncall-$TS_SYNCALL"
  RESP_JSON="$OUT_DIR/sync_all_response.json"
  mkdir -p "$OUT_DIR"

  echo "[INFO] 全量同步：POST $SERVER_URL/v1/refill/sync-all"
  curl -sS --connect-timeout "$TOPUP_CONNECT_TIMEOUT" --max-time "$TOPUP_MAX_TIME" -X POST "$SERVER_URL/v1/refill/sync-all" \
    -H "X-User-Key: $USER_KEY" \
    -H "Content-Type: application/json" \
    --data-binary "{}" > "$RESP_JSON"

  if ! count_new="$(json_topup_write_accounts_from_response "$RESP_JSON" "$ACCOUNTS_DIR" 2>/dev/null)"; then
    if grep -qi '"error"[[:space:]]*:[[:space:]]*"not-found"\|"error"[[:space:]]*:[[:space:]]*"not_found"' "$RESP_JSON" 2>/dev/null; then
      echo "[ERROR] sync-all 接口不存在：请先部署最新版服务端（包含 /v1/refill/sync-all）"
      exit 2
    fi
    echo "[ERROR] sync-all failed（原始响应如下）:"
    cat "$RESP_JSON" || true
    exit 2
  fi
  echo "[INFO] 已同步账号：$count_new"
  sync_managed_json
  echo "[OK] 全量同步完成"
  exit 0
fi

total=0
probed_ok=0
net_fail=0
invalid=0

probe_one() {
  local f="$1"
  local base
  base="$(basename "$f")"

  local type token aid email
  {
    IFS= read -r type || true
    IFS= read -r token || true
    IFS= read -r aid || true
    IFS= read -r email || true
  } < <(json_auth_fields4 "$f" 2>/dev/null || true)

  [[ "$type" != "codex" ]] && return 0
  [[ -z "$token" ]] && return 0

  local status wham_body
  wham_body="$OUT_DIR/.wham.$$.tmp"
  if [[ -n "$aid" ]]; then
    status="$(curl -sS --connect-timeout "$WHAM_CONNECT_TIMEOUT" --max-time "$WHAM_MAX_TIME" -o "$wham_body" -w "%{http_code}" \
      -H "Authorization: Bearer $token" \
      -H "Chatgpt-Account-Id: $aid" \
      -H "Accept: application/json, text/plain, */*" \
      -H "User-Agent: codex_cli_rs/0.76.0 (macOS/Linux)" \
      "https://chatgpt.com/backend-api/wham/usage" || true)"
  else
    status="$(curl -sS --connect-timeout "$WHAM_CONNECT_TIMEOUT" --max-time "$WHAM_MAX_TIME" -o "$wham_body" -w "%{http_code}" \
      -H "Authorization: Bearer $token" \
      -H "Accept: application/json, text/plain, */*" \
      -H "User-Agent: codex_cli_rs/0.76.0 (macOS/Linux)" \
      "https://chatgpt.com/backend-api/wham/usage" || true)"
  fi

  status="$(json_normalize_wham_status "$status" "$wham_body")"
  rm -f "$wham_body" 2>/dev/null || true

  if ! [[ "$status" =~ ^[0-9]{3}$ ]] || [[ "$status" == "000" ]]; then
    net_fail=$((net_fail+1))
    echo "$base" >> "$NETFAIL_LOG"
    return 0
  fi
  probed_ok=$((probed_ok+1))

  local probed_at
  probed_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

  local ident
  if [[ -n "$email" ]]; then
    ident="email:${email,,}"
  else
    ident="account_id:$aid"
  fi

  local email_hash
  if command -v sha256sum >/dev/null 2>&1; then
    email_hash="$(printf "%s" "$ident" | sha256sum | awk '{print $1}')"
  elif command -v shasum >/dev/null 2>&1; then
    email_hash="$(printf "%s" "$ident" | shasum -a 256 | awk '{print $1}')"
  else
    email_hash="$(printf "%s" "$ident" | openssl dgst -sha256 | awk '{print $NF}')"
  fi

  printf '{"file_name":"%s","email_hash":"%s","account_id":"%s","status_code":%s,"probed_at":"%s"}\n' \
    "$base" "$email_hash" "$aid" "$status" "$probed_at" >> "$REPORT_JSONL"

  if [[ "$status" == "401" || "$status" == "429" ]]; then
    invalid=$((invalid+1))
  fi
}

for f in "$ACCOUNTS_DIR"/*.json; do
  [[ -f "$f" ]] || continue
  total=$((total+1))
  probe_one "$f"
done

available_est=$((total - invalid))
hold_limit="$TOTAL_HOLD_LIMIT"
if ! [[ "$hold_limit" =~ ^[0-9]+$ ]] || [[ "$hold_limit" -lt 1 ]]; then
  hold_limit=50
fi

request_target="$TARGET_POOL_SIZE"
if [[ "$hold_limit" -gt "$request_target" ]]; then
  request_target="$hold_limit"
fi

echo "[INFO] 统计：total=$total available_est=$available_est probed_ok=$probed_ok net_fail=$net_fail invalid(401/429)=$invalid hold_limit=$hold_limit request_target=$request_target"

# 规则：网络失败(net_fail)默认按“可用”计入 available_est（即不算 invalid）
need_trigger=0
if [[ "$invalid" -gt 0 ]]; then need_trigger=1; fi
if [[ "$total" -lt "$TARGET_POOL_SIZE" ]]; then need_trigger=1; fi
if [[ "$total" -lt "$hold_limit" ]]; then need_trigger=1; fi

if [[ "$need_trigger" == "0" ]]; then
  echo "[OK] 未达到续杯条件：无需 topup"
  exit 0
fi

# jsonl -> reports array（纯 bash 拼接，避免 jq 依赖）
{
  printf '{"target_pool_size":%s,"reports":[' "${request_target}"
  first=1
  while IFS= read -r line; do
    [[ -z "$line" ]] && continue
    if [[ "$first" == "1" ]]; then
      first=0
    else
      printf ','
    fi
    printf '%s' "$line"
  done < "$REPORT_JSONL"
  printf ']}\n'
} > "$BODY_JSON"

echo "[INFO] 触发 topup：POST $SERVER_URL/v1/refill/topup"
curl -sS --connect-timeout "$TOPUP_CONNECT_TIMEOUT" --max-time "$TOPUP_MAX_TIME" -X POST "$SERVER_URL/v1/refill/topup" \
  -H "X-User-Key: $USER_KEY" \
  -H "Content-Type: application/json" \
  --data-binary "@$BODY_JSON" > "$RESP_JSON"

# 写入新账号（同时做 ok 校验）
if ! count_new="$(json_topup_write_accounts_from_response "$RESP_JSON" "$ACCOUNTS_DIR" 2>/dev/null)"; then
  echo "[ERROR] topup failed（原始响应如下）:"
  cat "$RESP_JSON" || true
  exit 2
fi

echo "[INFO] 写入新账号：$count_new"

# 删除失效文件（401/429），先备份（从 reports.jsonl 解析，避免 jq 依赖）
while IFS= read -r line; do
  [[ -z "$line" ]] && continue
  case "$line" in
    *'"status_code":401'*|*'"status_code":429'*)
      fn="$(printf "%s" "$line" | sed -E 's/.*"file_name":"([^"]+)".*/\1/')"
      [[ -z "$fn" ]] && continue
      if [[ -f "$ACCOUNTS_DIR/$fn" ]]; then
        cp -f "$ACCOUNTS_DIR/$fn" "$BACKUP_DIR/$fn" 2>/dev/null || true
        rm -f "$ACCOUNTS_DIR/$fn" 2>/dev/null || true
      fi
      ;;
  esac
done < "$REPORT_JSONL"

sync_managed_json

echo "[OK] 已完成单次续杯：新账号已写入 accounts-dir；失效(401/429/周配额耗尽映射429)文件已备份并删除。"
if [[ -n "$SYNC_TARGET_DIR" ]]; then
  echo "[OK] 已同步 managed json 到：$SYNC_TARGET_DIR"
fi
echo "      输出：$OUT_DIR"
