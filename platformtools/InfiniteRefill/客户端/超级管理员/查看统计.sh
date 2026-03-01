#!/usr/bin/env bash
set -euo pipefail

服务器地址="${1:-}"
管理员令牌="${2:-}"
管理员护卫码="${3:-}"

if [[ -z "$服务器地址" || -z "$管理员令牌" || -z "$管理员护卫码" ]]; then
  echo "用法：$0 服务器地址 管理员令牌 管理员护卫码"
  echo "示例：$0 https://127.0.0.1:8787 dev_admin_token_123 my-strong-guard-string"
  exit 1
fi

curl -sS "$服务器地址/admin/stats" \
  -H "Authorization: Bearer $管理员令牌" \
  -H "X-Admin-Guard: $管理员护卫码"
