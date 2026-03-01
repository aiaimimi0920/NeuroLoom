#!/usr/bin/env sh
set -eu

服务器地址="${1:-}"
管理员令牌="${2:-}"
管理员护卫码="${3:-}"
数量="${4:-1}"
标签="${5:-}"

if [ -z "$服务器地址" ] || [ -z "$管理员令牌" ] || [ -z "$管理员护卫码" ]; then
  echo "用法：$(basename "$0") 服务器地址 管理员令牌 管理员护卫码 数量 [标签]" >&2
  echo "示例：$(basename "$0") https://127.0.0.1:8787 dev_admin_token_123 my-strong-guard-string 30 2026-02-special" >&2
  exit 1
fi

json="{\"type\":\"upload\",\"count\":${数量},\"label\":\"${标签}\",\"bind_pool_size\":10}"

curl -sS -X POST "${服务器地址}/admin/keys/issue" \
  -H "Authorization: Bearer ${管理员令牌}" \
  -H "X-Admin-Guard: ${管理员护卫码}" \
  -H "Content-Type: application/json" \
  --data "${json}"

echo ""
