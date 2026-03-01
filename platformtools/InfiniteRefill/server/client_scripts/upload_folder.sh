#!/usr/bin/env sh
# 普通管理员（UPLOAD_KEY）客户端脚本：批量处理目录下的 json
#
# 依赖：curl
# - register 模式：无需 jq（仅用文件名推断 email）
# - probe 模式：需要 jq

set -eu

SERVER=""
UPLOAD_KEY=""
DIR=""
GLOB="*.json"
MODE="probe"
TIMEOUT=15
LIMIT=2000

while [ $# -gt 0 ]; do
  case "$1" in
    --server) SERVER="$2"; shift 2;;
    --upload-key) UPLOAD_KEY="$2"; shift 2;;
    --dir) DIR="$2"; shift 2;;
    --glob) GLOB="$2"; shift 2;;
    --mode) MODE="$2"; shift 2;;
    --timeout) TIMEOUT="$2"; shift 2;;
    --limit) LIMIT="$2"; shift 2;;
    *) echo "unknown arg: $1"; exit 2;;
  esac
done

if [ -z "$SERVER" ] || [ -z "$UPLOAD_KEY" ] || [ -z "$DIR" ]; then
  echo "[ERROR] --server/--upload-key/--dir required" >&2
  exit 2
fi

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
SINGLE="$SCRIPT_DIR/upload_single.sh"

ok=0
bad=0
count=0

# shellcheck disable=SC2086
for f in "$DIR"/$GLOB; do
  if [ ! -f "$f" ]; then
    continue
  fi
  count=$((count+1))
  if [ "$LIMIT" -gt 0 ] && [ "$count" -gt "$LIMIT" ]; then
    break
  fi

  if "$SINGLE" --server "$SERVER" --upload-key "$UPLOAD_KEY" --file "$f" --mode "$MODE" --timeout "$TIMEOUT"; then
    ok=$((ok+1))
  else
    bad=$((bad+1))
  fi
done

echo "done ok=$ok bad=$bad total=$((ok+bad))"
if [ "$bad" -gt 0 ]; then
  exit 2
fi
exit 0
