#!/usr/bin/env sh
set -eu

服务器地址="${1:-}"
管理员令牌="${2:-}"
管理员护卫码="${3:-}"
输出文件="${4:-backup_dump.json}"

if [ -z "$服务器地址" ] || [ -z "$管理员令牌" ] || [ -z "$管理员护卫码" ]; then
  echo "用法：$(basename "$0") 服务器地址 管理员令牌 管理员护卫码 [输出文件]" >&2
  echo "示例：$(basename "$0") https://127.0.0.1:8787 dev_admin_token_123 my-strong-guard-string backup_dump.json" >&2
  exit 1
fi

curl -sS "${服务器地址}/admin/backup/export" \
  -H "Authorization: Bearer ${管理员令牌}" \
  -H "X-Admin-Guard: ${管理员护卫码}" \
  -H "Content-Type: application/json" \
  > "${输出文件}"

echo "[OK] 已写入：${输出文件}"
