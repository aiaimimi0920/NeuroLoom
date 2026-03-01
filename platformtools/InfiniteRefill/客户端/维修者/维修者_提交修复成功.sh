#!/usr/bin/env bash
set -euo pipefail

# 维修者：提交修复成功（POST /v1/repairs/submit-fixed）
# 目标：尽量不依赖额外工具（macOS 优先用系统自带 osascript/JXA 生成请求体）。
#
# 用法：
#   bash 维修者_提交修复成功.sh 服务器地址 维修者密钥 artwork_id 修复后json路径

SERVER_URL="${1:-}"
REPAIRER_KEY="${2:-}"
ARTWORK_ID="${3:-}"
JSON_FILE="${4:-}"

if [[ -z "$SERVER_URL" || -z "$REPAIRER_KEY" || -z "$ARTWORK_ID" || -z "$JSON_FILE" ]]; then
  echo "用法：$0 服务器地址 维修者密钥 artwork_id 修复后json路径" >&2
  echo "示例：$0 http://127.0.0.1:8788 k_xxx acc_demo_1 ./fixed.json" >&2
  exit 1
fi

if [[ ! -f "$JSON_FILE" ]]; then
  echo "[ERROR] 文件不存在：$JSON_FILE" >&2
  exit 2
fi

# 生成请求体 JSON：{artwork_id, fixed_artwork}
make_body() {
  if command -v osascript >/dev/null 2>&1; then
    # macOS：系统自带 JXA
    ARTWORK_ID="$ARTWORK_ID" JSON_FILE="$JSON_FILE" osascript -l JavaScript <<'JXA'
ObjC.import('Foundation');

function readUtf8(path) {
  const ns = $.NSString.stringWithContentsOfFileEncodingError($(path), $.NSUTF8StringEncoding, null);
  if (!ns) throw new Error('read_file_failed');
  return ObjC.unwrap(ns);
}

const artworkId = $.getenv('ARTWORK_ID');
const filePath = $.getenv('JSON_FILE');
const text = readUtf8(filePath);
const art = JSON.parse(text);

const body = { artwork_id: artworkId, fixed_artwork: art };
console.log(JSON.stringify(body));
JXA
    return
  fi

  if command -v jq >/dev/null 2>&1; then
    jq -n --arg artwork_id "$ARTWORK_ID" --slurpfile art "$JSON_FILE" '{artwork_id:$artwork_id, fixed_artwork:$art[0]}'
    return
  fi

  if command -v python3 >/dev/null 2>&1; then
    ARTWORK_ID="$ARTWORK_ID" JSON_FILE="$JSON_FILE" python3 - <<'PY'
import json, os
artwork_id = os.environ['ARTWORK_ID']
path = os.environ['JSON_FILE']
with open(path, 'r', encoding='utf-8') as f:
    art = json.load(f)
print(json.dumps({'artwork_id': artwork_id, 'fixed_artwork': art}, ensure_ascii=False))
PY
    return
  fi

  echo "[ERROR] 无法生成请求体：缺少 osascript/jq/python3 之一。" >&2
  exit 2
}

make_body \
  | curl -sS -X POST "$SERVER_URL/v1/repairs/submit-fixed" \
      -H "X-Upload-Key: $REPAIRER_KEY" \
      -H "Content-Type: application/json" \
      --data-binary @-
