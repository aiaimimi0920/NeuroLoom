#!/usr/bin/env sh
# 普通管理员（UPLOAD_KEY）客户端脚本：上传单个认证文件（合规版：不上传 token）。
#
# 依赖：curl
# - register 模式：只用文件名推断 email（xxx@email.json），无需 jq
# - probe 模式：需要 jq 解析 access_token/account_id（若无 jq 会拒绝并提示改用 register）
#
# 用法：
#   ./upload_single.sh --server http://127.0.0.1:8789 --upload-key test_upload_key --file "/path/to/xxx@email.json" --mode register
#   ./upload_single.sh --server http://127.0.0.1:8789 --upload-key test_upload_key --file "/path/to/xxx@email.json" --mode probe

set -eu

SERVER=""
UPLOAD_KEY=""
FILE=""
MODE="probe"
TIMEOUT=15

while [ $# -gt 0 ]; do
  case "$1" in
    --server) SERVER="$2"; shift 2;;
    --upload-key) UPLOAD_KEY="$2"; shift 2;;
    --file) FILE="$2"; shift 2;;
    --mode) MODE="$2"; shift 2;;
    --timeout) TIMEOUT="$2"; shift 2;;
    *) echo "unknown arg: $1"; exit 2;;
  esac
done

if [ -z "$SERVER" ] || [ -z "$UPLOAD_KEY" ] || [ -z "$FILE" ]; then
  echo "[ERROR] --server/--upload-key/--file required" >&2
  exit 2
fi

utc_now() {
  # ISO 8601 UTC
  date -u "+%Y-%m-%dT%H:%M:%SZ"
}

sha256_hex() {
  # 尽量兼容 macOS/Linux
  if command -v shasum >/dev/null 2>&1; then
    printf "%s" "$1" | shasum -a 256 | awk '{print $1}'
  elif command -v openssl >/dev/null 2>&1; then
    # openssl 输出格式可能为: (stdin)= <hex>
    printf "%s" "$1" | openssl dgst -sha256 | awk '{print $NF}'
  else
    echo "[ERROR] need shasum or openssl for sha256" >&2
    exit 2
  fi
}

basename_noext() {
  b=$(basename "$1")
  case "$b" in
    *.json|*.JSON) printf "%s" "${b%.*}" ;;
    *) printf "%s" "$b" ;;
  esac
}

infer_email() {
  b=$(basename_noext "$1")
  case "$b" in
    *@*) printf "%s" "$b";;
    *) printf "%s" "";;
  esac
}

email_hash() {
  email="$1"
  account_id="$2"
  if [ -n "$email" ]; then
    sha256_hex "email:$(printf "%s" "$email" | tr '[:upper:]' '[:lower:]')"
  else
    sha256_hex "account_id:$account_id"
  fi
}

post_json() {
  path="$1"
  data="$2"
  curl -sS -i \
    -m "$TIMEOUT" \
    -H "Content-Type: application/json" \
    -H "X-Upload-Key: $UPLOAD_KEY" \
    -X POST "$SERVER$path" \
    --data "$data"
}

EMAIL=$(infer_email "$FILE")
ACCOUNT_ID=""
EHASH=$(email_hash "$EMAIL" "$ACCOUNT_ID")
NOW=$(utc_now)

if [ "$MODE" = "register" ]; then
  payload=$(printf '{"accounts":[{"email_hash":"%s","account_id":"%s","seen_at":"%s"}]}' "$EHASH" "$ACCOUNT_ID" "$NOW")
  post_json "/v1/accounts/register" "$payload"
  exit 0
fi

# probe 模式需要 jq
if ! command -v jq >/dev/null 2>&1; then
  echo "[ERROR] probe mode requires jq. Install jq or use --mode register" >&2
  exit 2
fi

ACCESS_TOKEN=$(jq -r '.access_token // empty' "$FILE")
ACCOUNT_ID=$(jq -r '.account_id // empty' "$FILE")
if [ -z "$EMAIL" ]; then
  EMAIL=$(jq -r '.email // empty' "$FILE")
fi
EHASH=$(email_hash "$EMAIL" "$ACCOUNT_ID")

if [ -z "$ACCESS_TOKEN" ]; then
  echo "[ERROR] missing access_token in file" >&2
  exit 2
fi

# 本地 wham probe
STATUS_CODE=$(curl -sS -o /dev/null -w "%{http_code}" -m "$TIMEOUT" \
  -H "Accept: application/json, text/plain, */*" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  ${ACCOUNT_ID:+-H "Chatgpt-Account-Id: $ACCOUNT_ID"} \
  "$whamURL" || true)

NOW=$(utc_now)
payload=$(printf '{"reports":[{"email_hash":"%s","account_id":"%s","status_code":%s,"probed_at":"%s"}]}' "$EHASH" "$ACCOUNT_ID" "$STATUS_CODE" "$NOW")
post_json "/v1/probe-report" "$payload"
