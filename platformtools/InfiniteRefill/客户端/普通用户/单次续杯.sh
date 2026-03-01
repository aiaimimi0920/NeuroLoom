#!/usr/bin/env bash
set -euo pipefail

# 单次续杯（探测 -> 上报状态 -> 触发 topup -> 写入新账号 -> 删除失效账号）
#
# 依赖：curl
# JSON 解析（任选其一）：python3（推荐）/ osascript(JXA, macOS 自带) / jq（可选兜底）
#
# 服务端契约：
# - POST /v1/refill/topup
#   Header: X-User-Key: <USER_KEY or UPLOAD_KEY>
#   Body:
#     {"target_pool_size":10,"reports":[{"file_name":"x.json","email_hash":"...","account_id":"...","status_code":401,"probed_at":"2026-..Z"}]}
#   Resp:
#     {"ok":true,"accounts":[{"file_name":"无限续杯-001.json","auth_json":{...}}, ...]}

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

LIB_SH="$ROOT_DIR/客户端/_lib/json.sh"
# shellcheck disable=SC1090
source "$LIB_SH"

need_cmd curl
need_json_parser

CFG_YAML="$ROOT_DIR/config.yaml"
CFG_ENV="$SCRIPT_DIR/状态/无限续杯配置.env"

SERVER_URL="${1:-}"
USER_KEY="${2:-}"
TARGET_POOL_SIZE=10
TRIGGER_REMAINING=2

if [[ -f "$CFG_ENV" ]]; then
  # shellcheck disable=SC1090
  source "$CFG_ENV" || true
fi

SERVER_URL="${SERVER_URL:-${SERVER_URL:-}}"
USER_KEY="${USER_KEY:-${USER_KEY:-}}"
TARGET_POOL_SIZE="${TARGET_POOL_SIZE:-10}"
TRIGGER_REMAINING="${TRIGGER_REMAINING:-2}"

if [[ -z "$SERVER_URL" || -z "$USER_KEY" ]]; then
  echo "[ERROR] 未配置 SERVER_URL/USER_KEY，请先运行：bash \"$SCRIPT_DIR/无限续杯.sh\""
  exit 2
fi

if [[ ! -f "$CFG_YAML" ]]; then
  echo "[ERROR] 找不到 config.yaml：$CFG_YAML"
  exit 3
fi

ACCOUNTS_DIR="$(grep -E '^[[:space:]]*auth-dir[[:space:]]*:' -m 1 "$CFG_YAML" | sed -E 's/^[[:space:]]*auth-dir[[:space:]]*:[[:space:]]*//; s/#.*$//; s/^["\x27]//; s/["\x27][[:space:]]*$//')"
if [[ -z "$ACCOUNTS_DIR" ]]; then
  echo "[ERROR] 无法从 $CFG_YAML 解析 auth-dir"
  exit 3
fi

TS="$(date -u +%Y%m%d-%H%M%S)"
OUT_DIR="$SCRIPT_DIR/状态/out/单次续杯-$TS"
REPORT_JSONL="$OUT_DIR/reports.jsonl"
BODY_JSON="$OUT_DIR/topup_body.json"
RESP_JSON="$OUT_DIR/topup_response.json"
BACKUP_DIR="$OUT_DIR/backup"

mkdir -p "$BACKUP_DIR"
: > "$REPORT_JSONL"

# managed set: prefer 无限续杯-*.json
managed_glob=("$ACCOUNTS_DIR"/无限续杯-*.json)
use_prefix=0
if [[ -e "${managed_glob[0]}" ]]; then
  use_prefix=1
fi

total=0
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

  local status
  if [[ -n "$aid" ]]; then
    status="$(curl -sS -o /dev/null -w "%{http_code}" \
      -H "Authorization: Bearer $token" \
      -H "Chatgpt-Account-Id: $aid" \
      -H "Accept: application/json, text/plain, */*" \
      -H "User-Agent: codex_cli_rs/0.76.0 (macOS/Linux)" \
      "https://chatgpt.com/backend-api/wham/usage" || true)"
  else
    status="$(curl -sS -o /dev/null -w "%{http_code}" \
      -H "Authorization: Bearer $token" \
      -H "Accept: application/json, text/plain, */*" \
      -H "User-Agent: codex_cli_rs/0.76.0 (macOS/Linux)" \
      "https://chatgpt.com/backend-api/wham/usage" || true)"
  fi

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

if [[ "$use_prefix" == "1" ]]; then
  echo "[INFO] 检测到前缀“无限续杯-*.json”，仅管理这些文件"
  for f in "$ACCOUNTS_DIR"/无限续杯-*.json; do
    [[ -f "$f" ]] || continue
    total=$((total+1))
    probe_one "$f"
  done
else
  echo "[WARN] 未检测到“无限续杯-*.json”，将退化为管理 auth-dir 下所有 codex 文件（可能包含你的其它文件）"
  for f in "$ACCOUNTS_DIR"/*.json; do
    [[ -f "$f" ]] || continue
    total=$((total+1))
    probe_one "$f"
  done
fi

threshold=$((TARGET_POOL_SIZE - TRIGGER_REMAINING))
if [[ "$threshold" -lt 1 ]]; then threshold=1; fi

echo "[INFO] 统计：total=$total invalid(401/429)=$invalid trigger_invalid>=$threshold"

need_trigger=0
if [[ "$total" -lt "$TARGET_POOL_SIZE" ]]; then need_trigger=1; fi
if [[ "$invalid" -ge "$threshold" ]]; then need_trigger=1; fi

if [[ "$need_trigger" == "0" ]]; then
  echo "[OK] 未达到续杯条件：无需 topup"
  exit 0
fi

# jsonl -> reports array（纯 bash 拼接，避免 jq 依赖）
{
  printf '{"target_pool_size":%s,"reports":[' "${TARGET_POOL_SIZE}"
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

echo "[OK] 已完成单次续杯：新账号已写入 auth-dir；失效(401/429)文件已备份并删除。"
echo "      输出：$OUT_DIR"
