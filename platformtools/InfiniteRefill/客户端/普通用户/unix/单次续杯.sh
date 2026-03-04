#!/usr/bin/env bash
set -euo pipefail

# 单次续杯（探测 -> 上报状态 -> 触发 topup -> 写入新账号 -> 删除失效账号）
#
# 依赖：curl
# JSON 解析（任选其一）：python3（推荐）/ osascript(JXA, macOS 自带) / jq（可选兜底）
#
# 服务端契约：
# - POST /v1/refill/topup
#   Header: X-User-Key: <USER_KEY>
#   Body:
#     {"target_pool_size":10,"reports":[{"file_name":"x.json","email_hash":"...","account_id":"...","status_code":401,"probed_at":"2026-..Z"}]}
#   Resp:
#     {"ok":true,"accounts":[{"file_name":"无限续杯-001.json","download_url":"https://..."}], ...}

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# 自动检测：全平台版本（在 unix/ 子目录）vs 分平台版本（在根目录）
if [ "$(basename "$SCRIPT_DIR")" = "unix" ]; then
  ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
else
  ROOT_DIR="$SCRIPT_DIR"
fi

LIB_SH="$SCRIPT_DIR/json.sh"
# shellcheck disable=SC1090
source "$LIB_SH"

need_cmd curl
need_json_parser

CFG_ENV="$ROOT_DIR/无限续杯配置.env"

MODE_SYNC_ALL=0
if [[ "${1:-}" == "--sync-all" ]]; then
  MODE_SYNC_ALL=1
  shift
fi

SERVER_URL="${1:-}"
USER_KEY="${2:-}"
ACCOUNTS_DIR="$ROOT_DIR/accounts"
TARGET_POOL_SIZE=10
TOTAL_HOLD_LIMIT=50
TRIGGER_REMAINING=2
SYNC_MODE=none
SYNC_TARGET_DIR=

if [[ -f "$CFG_ENV" ]]; then
  # shellcheck disable=SC1090
  source "$CFG_ENV" || true
fi

SERVER_URL="${SERVER_URL:-${SERVER_URL:-}}"
USER_KEY="${USER_KEY:-${USER_KEY:-}}"
ACCOUNTS_DIR="${ACCOUNTS_DIR:-$ROOT_DIR/accounts}"
TARGET_POOL_SIZE="${TARGET_POOL_SIZE:-10}"
TOTAL_HOLD_LIMIT="${TOTAL_HOLD_LIMIT:-50}"
TRIGGER_REMAINING="${TRIGGER_REMAINING:-2}"
SYNC_MODE="${SYNC_MODE:-none}"
SYNC_TARGET_DIR="${SYNC_TARGET_DIR:-}"

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
OUT_DIR="$ROOT_DIR/out/单次续杯-$TS"
REPORT_JSONL="$OUT_DIR/reports.jsonl"
BODY_JSON="$OUT_DIR/topup_body.json"
RESP_JSON="$OUT_DIR/topup_response.json"
BACKUP_DIR="$OUT_DIR/backup"
NETFAIL_LOG="$OUT_DIR/probe_netfail.log"

mkdir -p "$BACKUP_DIR"
: > "$REPORT_JSONL"
: > "$NETFAIL_LOG"

sync_managed_json() {
  local mode target
  mode="$(printf '%s' "$SYNC_MODE" | tr '[:upper:]' '[:lower:]')"
  target="$SYNC_TARGET_DIR"

  [[ "$mode" == "none" || -z "$target" ]] && return 0
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
    if [[ "$mode" == "symlink" ]]; then
      if [[ -L "$tp" ]]; then
        rm -f "$tp" 2>/dev/null || true
      elif [[ -e "$tp" ]]; then
        continue
      fi
      ln -s "$f" "$tp" 2>/dev/null || true
    fi
  done

  printf "%s\n" "${names[@]}" > "$manifest"
}

if [[ "$MODE_SYNC_ALL" == "1" ]]; then
  TS_SYNCALL="$(date -u +%Y%m%d-%H%M%S)"
  OUT_DIR="${TMPDIR:-/tmp}/InfiniteRefill-syncall-$TS_SYNCALL"
  RESP_JSON="$OUT_DIR/sync_all_response.json"
  mkdir -p "$OUT_DIR"

  echo "[INFO] 全量同步：POST $SERVER_URL/v1/refill/sync-all"
  curl -sS -X POST "$SERVER_URL/v1/refill/sync-all" \
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

# managed set: prefer 无限续杯-*.json
managed_glob=("$ACCOUNTS_DIR"/无限续杯-*.json)
use_prefix=0
if [[ -e "${managed_glob[0]}" ]]; then
  use_prefix=1
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
    status="$(curl -sS -o "$wham_body" -w "%{http_code}" \
      -H "Authorization: Bearer $token" \
      -H "Chatgpt-Account-Id: $aid" \
      -H "Accept: application/json, text/plain, */*" \
      -H "User-Agent: codex_cli_rs/0.76.0 (macOS/Linux)" \
      "https://chatgpt.com/backend-api/wham/usage" || true)"
  else
    status="$(curl -sS -o "$wham_body" -w "%{http_code}" \
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
    ident="email:$(printf '%s' "$email" | tr '[:upper:]' '[:lower:]')"
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

  local replay_from_confidence=false
  if [[ -f "$f.replay" || -f "$f.replay_failed_once" ]]; then
    replay_from_confidence=true
  fi

  printf '{"file_name":"%s","email_hash":"%s","account_id":"%s","status_code":%s,"probed_at":"%s","replay_from_confidence":%s}\n' \
    "$base" "$email_hash" "$aid" "$status" "$probed_at" "$replay_from_confidence" >> "$REPORT_JSONL"

  if [[ "$status" == "401" || "$status" == "429" ]]; then
    invalid=$((invalid+1))
  fi
}

if [[ "$use_prefix" == "1" ]]; then
  echo "[INFO] 检测到前缀“无限续杯-*.json”，仅管理这些文件"
  for f in "$ACCOUNTS_DIR"/无限续杯-*.json; do
    [[ -f "$f" ]] || continue
    total=$((total+1))
    probe_one "$f"
  done
else
  echo "[WARN] 未检测到“无限续杯-*.json”，将退化为管理 accounts-dir 下所有 codex 文件（可能包含你的其它文件）"
  for f in "$ACCOUNTS_DIR"/*.json; do
    [[ -f "$f" ]] || continue
    total=$((total+1))
    probe_one "$f"
  done
fi

available_est=$((total - invalid))

# 计算 hold_limit
hold_limit="${TOTAL_HOLD_LIMIT:-50}"
if ! [[ "$hold_limit" =~ ^[0-9]+$ ]] || [[ "$hold_limit" -lt 1 ]]; then hold_limit=50; fi

# REQUEST_TARGET = hold_limit - available_est（精确补差，不超发）
request_target=$((hold_limit - available_est))
if [[ "$request_target" -lt 0 ]]; then request_target=0; fi

echo "[INFO] 统计：total=$total available_est=$available_est probed_ok=$probed_ok net_fail=$net_fail invalid(401/429)=$invalid hold_limit=$hold_limit request_target=$request_target"

# 触发规则
need_trigger=0
if [[ "$invalid" -gt 0 ]]; then need_trigger=1; fi
if [[ "$total" -lt "$TARGET_POOL_SIZE" ]]; then need_trigger=1; fi
if [[ "$hold_limit" -gt 0 ]] && [[ "$available_est" -lt "$hold_limit" ]]; then need_trigger=1; fi

if [[ "$need_trigger" == "0" ]]; then
  echo "[OK] 未达到续杯条件：无需 topup"
  exit 0
fi

# 持有量已达上限时提前退出
if [[ "$request_target" -eq 0 ]]; then
  echo "[OK] 无需续杯：持有量已达上限（available_est=$available_est hold_limit=$hold_limit）"
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
curl -sS -X POST "$SERVER_URL/v1/refill/topup" \
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

# 为“待置信区回放账号”写本地标记（用于下一轮上报 replay_from_confidence）
python3 - <<'PY' "$RESP_JSON" "$ACCOUNTS_DIR" 2>/dev/null || true
import json, os, sys
resp_file, accounts_dir = sys.argv[1], sys.argv[2]
try:
    with open(resp_file, 'r', encoding='utf-8') as f:
        obj = json.load(f)
    for acc in obj.get('accounts', []) or []:
        if acc.get('replay_from_confidence') is True:
            fn = str(acc.get('file_name') or '').strip()
            if not fn:
                continue
            p = os.path.join(accounts_dir, fn + '.replay')
            with open(p, 'w', encoding='utf-8') as w:
                w.write('1\n')
except Exception:
    pass
PY

# 删除失效文件（401/429），回放账号保留一次观察窗口
while IFS= read -r line; do
  [[ -z "$line" ]] && continue
  case "$line" in
    *'"status_code":401'*|*'"status_code":429'*)
      fn="$(printf "%s" "$line" | sed -E 's/.*"file_name":"([^"]+)".*/\1/')"
      [[ -z "$fn" ]] && continue
      if [[ -f "$ACCOUNTS_DIR/$fn" ]]; then
        if [[ -f "$ACCOUNTS_DIR/$fn.replay" && ! -f "$ACCOUNTS_DIR/$fn.replay_failed_once" ]]; then
          mv -f "$ACCOUNTS_DIR/$fn.replay" "$ACCOUNTS_DIR/$fn.replay_failed_once" 2>/dev/null || true
          continue
        fi
        cp -f "$ACCOUNTS_DIR/$fn" "$BACKUP_DIR/$fn" 2>/dev/null || true
        rm -f "$ACCOUNTS_DIR/$fn" 2>/dev/null || true
        rm -f "$ACCOUNTS_DIR/$fn.replay" "$ACCOUNTS_DIR/$fn.replay_failed_once" 2>/dev/null || true
      fi
      ;;
  esac
done < "$REPORT_JSONL"

sync_managed_json

echo "[OK] 已完成单次续杯：新账号已写入 accounts-dir；失效(401/429/周配额耗尽映射429)文件已备份并删除。"
if [[ "$(printf '%s' "$SYNC_MODE" | tr '[:upper:]' '[:lower:]')" != "none" && -n "$SYNC_TARGET_DIR" ]]; then
  echo "[OK] 已同步 managed json 到：${SYNC_TARGET_DIR}（mode=${SYNC_MODE}）"
fi
echo "      输出：$OUT_DIR"