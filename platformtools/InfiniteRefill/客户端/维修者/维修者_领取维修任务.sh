#!/usr/bin/env bash
set -euo pipefail

# 维修者：领取维修区任务（POST /v1/repairs/claim）
# 目标：不依赖 jq。
#
# 用法：
#   bash 维修者_领取维修任务.sh 服务器地址 维修者密钥 [count]

SERVER_URL="${1:-}"
REPAIRER_KEY="${2:-}"
COUNT="${3:-1}"

if [[ -z "$SERVER_URL" || -z "$REPAIRER_KEY" ]]; then
  echo "用法：$0 服务器地址 维修者密钥 [count]" >&2
  echo "示例：$0 http://127.0.0.1:8788 k_xxx 1" >&2
  exit 1
fi

curl -sS -X POST "$SERVER_URL/v1/repairs/claim" \
  -H "X-Upload-Key: $REPAIRER_KEY" \
  -H "Content-Type: application/json" \
  -d "{\"count\":$COUNT}"
