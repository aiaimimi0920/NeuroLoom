#!/usr/bin/env sh
set -eu

服务器地址="${1:-}"
管理员令牌="${2:-}"
管理员护卫码="${3:-}"
备份文件="${4:-}"

if [ -z "$服务器地址" ] || [ -z "$管理员令牌" ] || [ -z "$管理员护卫码" ] || [ -z "$备份文件" ]; then
  echo "用法：$(basename "$0") 服务器地址 管理员令牌 管理员护卫码 备份文件" >&2
  echo "示例：$(basename "$0") https://127.0.0.1:8787 dev_admin_token_123 my-strong-guard-string backup_dump.json" >&2
  exit 1
fi

curl -sS -X POST "${服务器地址}/admin/backup/import" \
  -H "Authorization: Bearer ${管理员令牌}" \
  -H "X-Admin-Guard: ${管理员护卫码}" \
  -H "Content-Type: application/json" \
  --data-binary "@${备份文件}"

echo ""
