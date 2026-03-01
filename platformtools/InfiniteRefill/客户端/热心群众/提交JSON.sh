#!/usr/bin/env bash
set -euo pipefail

# 热心群众：提交 JSON（支持文件/文件夹；带合规检查）
#
# 用法：
#   bash ./提交JSON.sh 服务器地址 上传密钥 接口 路径
#
# 接口：register | report
# 路径：单个 .json 文件，或包含多个 .json 的文件夹
#
# 合规检查：
# - 必须是合法 JSON
# - register：必须包含 accounts 数组；每项至少 email_hash(64hex)/seen_at
# - report：必须包含 reports 数组；每项至少 email_hash(64hex)/status_code/probed_at
# - 已禁用“敏感字段”拦截：允许包含 access_token/refresh_token/id_token
#
# 依赖：curl
# JSON 解析（任选其一）：python3（推荐）/ osascript(JXA, macOS 自带) / jq（可选兜底）

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

LIB_SH="$ROOT_DIR/客户端/_lib/json.sh"
# shellcheck disable=SC1090
source "$LIB_SH"

need_cmd curl
need_json_parser

服务器地址="${1:-}"
上传密钥="${2:-}"
接口="${3:-}"
路径="${4:-}"

if [[ -z "$服务器地址" || -z "$上传密钥" || -z "$接口" || -z "$路径" ]]; then
  echo "用法：$0 服务器地址 上传密钥 接口 路径"
  echo "接口：register | report"
  echo "路径：单文件.json 或 文件夹"
  exit 1
fi

接口_lc="${接口,,}"
case "$接口_lc" in
  register) url_path="/v1/accounts/register" ;;
  report)   url_path="/v1/probe-report" ;;
  *) echo "[ERROR] 接口参数只能是 register 或 report"; exit 2 ;;
 esac

check_json() {
  local f="$1"
  json_check_payload_file "$接口_lc" "$f"
}

post_one() {
  local f="$1"
  local base
  base="$(basename "$f")"

  local msg
  if ! msg="$(check_json "$f" 2>&1)"; then
    echo "[SKIP] $base：$msg"
    return 0
  fi

  echo "[POST] $base -> $url_path"
  curl -sS -X POST "$服务器地址$url_path" \
    -H "X-Upload-Key: $上传密钥" \
    -H "Content-Type: application/json" \
    --data-binary "@$f" >/dev/null
}

total=0
ok=0
bad=0

if [[ -d "$路径" ]]; then
  echo "[INFO] 模式：文件夹"
  shopt -s nullglob
  for f in "$路径"/*.json; do
    total=$((total+1))
    if post_one "$f"; then ok=$((ok+1)); else bad=$((bad+1)); fi
  done
else
  echo "[INFO] 模式：单文件"
  total=1
  if post_one "$路径"; then ok=1; else bad=1; fi
fi

echo
echo "[DONE] total=$total ok=$ok bad=$bad"
