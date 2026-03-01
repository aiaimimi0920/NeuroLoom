#!/usr/bin/env bash
set -euo pipefail

# 维修者：误报恢复（POST /v1/repairs/submit-misreport）
# 目标：尽量不依赖额外工具（macOS 优先用系统自带 osascript/JXA 生成请求体）。
#
# 用法：
#   bash 维修者_误报恢复.sh 服务器地址 维修者密钥 artwork_id [note]

SERVER_URL="${1:-}"
REPAIRER_KEY="${2:-}"
ARTWORK_ID="${3:-}"
NOTE="${4:-}"

if [[ -z "$SERVER_URL" || -z "$REPAIRER_KEY" || -z "$ARTWORK_ID" ]]; then
  echo "用法：$0 服务器地址 维修者密钥 artwork_id [note]" >&2
  echo "示例：$0 http://127.0.0.1:8788 k_xxx acc_demo_1 '误报full'" >&2
  exit 1
fi

make_body() {
  if command -v osascript >/dev/null 2>&1; then
    ARTWORK_ID="$ARTWORK_ID" NOTE="$NOTE" osascript -l JavaScript <<'JXA'
const artworkId = $.getenv('ARTWORK_ID');
const note = $.getenv('NOTE') || '';
console.log(JSON.stringify({ artwork_id: artworkId, note }));
JXA
    return
  fi

  if command -v jq >/dev/null 2>&1; then
    jq -n --arg artwork_id "$ARTWORK_ID" --arg note "$NOTE" '{artwork_id:$artwork_id, note:$note}'
    return
  fi

  if command -v python3 >/dev/null 2>&1; then
    ARTWORK_ID="$ARTWORK_ID" NOTE="$NOTE" python3 - <<'PY'
import json, os
print(json.dumps({'artwork_id': os.environ['ARTWORK_ID'], 'note': os.environ.get('NOTE','')}, ensure_ascii=False))
PY
    return
  fi

  echo "[ERROR] 无法生成请求体：缺少 osascript/jq/python3 之一。" >&2
  exit 2
}

make_body \
  | curl -sS -X POST "$SERVER_URL/v1/repairs/submit-misreport" \
      -H "X-Upload-Key: $REPAIRER_KEY" \
      -H "Content-Type: application/json" \
      --data-binary @-
