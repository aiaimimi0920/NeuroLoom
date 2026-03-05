#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
LOCK_FILE="$SCRIPT_DIR/_task.lock"
MAX_LOCK_AGE=600

if [[ -f "$LOCK_FILE" ]]; then
  now="$(date +%s)"
  old="$(stat -c %Y "$LOCK_FILE" 2>/dev/null || stat -f %m "$LOCK_FILE" 2>/dev/null || echo 0)"
  if [[ "$old" =~ ^[0-9]+$ ]] && (( now - old < MAX_LOCK_AGE )); then
    exit 0
  fi
  rm -f "$LOCK_FILE" 2>/dev/null || true
fi

trap 'rm -f "$LOCK_FILE" >/dev/null 2>&1 || true' EXIT
printf '%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$LOCK_FILE"

bash "$SCRIPT_DIR/_内部_自动清理.sh" apply >/tmp/自动清理.log 2>&1 || true

set +e
bash "$SCRIPT_DIR/单次续杯.sh" --from-task >/tmp/无限续杯.log 2>&1
EC=$?
set -e

if [[ "$EC" == "4" || "$EC" == "5" ]]; then
  bash "$SCRIPT_DIR/无限续杯.sh" --disable-task-silent >/dev/null 2>&1 || true
fi

exit "$EC"
