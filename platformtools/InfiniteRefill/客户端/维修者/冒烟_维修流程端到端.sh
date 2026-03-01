#!/usr/bin/env bash
set -euo pipefail

# 维修者流程端到端冒烟（本地 wrangler dev / 线上均可）
#
# 目标验证（对应 TODO #18）：
# - submit(account_id) -> claim -> report-damage(full) -> repairs/claim -> submit-failed ×3 -> graveyard + tombstone -> 再 submit(同account_id) 被拒绝
#
# 用法：
#   bash ./客户端/维修者/冒烟_维修流程端到端.sh <SERVER_URL> <UPLOAD_KEY>
#
# 依赖：curl
# JSON 解析（任选其一）：python3（推荐）/ osascript(JXA, macOS 自带) / jq（兜底）

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

LIB_SH="$ROOT_DIR/客户端/_lib/json.sh"
# shellcheck disable=SC1090
source "$LIB_SH"

need_cmd curl
need_json_parser

SERVER_URL="${1:-}"
UPLOAD_KEY="${2:-}"

if [[ -z "$SERVER_URL" || -z "$UPLOAD_KEY" ]]; then
  echo "用法：$0 <SERVER_URL> <UPLOAD_KEY>" >&2
  exit 2
fi

curl_json() {
  local method="$1"
  local path="$2"
  local body="$3"

  curl -sS -X "$method" "$SERVER_URL$path" \
    -H "X-Upload-Key: $UPLOAD_KEY" \
    -H "Content-Type: application/json" \
    --data-binary "$body"
}

# 从 JSON 中取值（支持：a.b.c；不存在则输出空行）
json_get() {
  local json_text="$1"
  local key_path="$2"

  if command -v python3 >/dev/null 2>&1; then
    python3 - "$key_path" <<'PY'
import json,sys
path=sys.argv[1]
obj=json.loads(sys.stdin.read() or '{}')
cur=obj
for part in path.split('.'):
  if isinstance(cur, dict) and part in cur:
    cur=cur[part]
  else:
    print('')
    sys.exit(0)
if cur is None:
  print('')
elif isinstance(cur,(dict,list)):
  print(json.dumps(cur, ensure_ascii=False, separators=(',',':')))
else:
  print(str(cur))
PY
    return 0
  fi

  if command -v osascript >/dev/null 2>&1; then
    KEY_PATH="$key_path" osascript -l JavaScript <<'JXA'
ObjC.import('Foundation');
const raw = ObjC.unwrap($.NSFileHandle.fileHandleWithStandardInput.readDataToEndOfFile);
let obj = {};
try { obj = JSON.parse(raw); } catch(e) { console.log(''); $.exit(0); }
const path = ObjC.unwrap($.getenv('KEY_PATH'));
let cur = obj;
for (const p of path.split('.')) {
  if (cur && typeof cur === 'object' && !Array.isArray(cur) && Object.prototype.hasOwnProperty.call(cur,p)) {
    cur = cur[p];
  } else {
    console.log('');
    $.exit(0);
  }
}
if (cur === null || cur === undefined) console.log('');
else if (typeof cur === 'object') console.log(JSON.stringify(cur));
else console.log(String(cur));
JXA
    return 0
  fi

  if command -v jq >/dev/null 2>&1; then
    # key_path: a.b.c -> .a.b.c
    jq -r ".${key_path} // empty" 2>/dev/null <<<"$json_text" || true
    return 0
  fi

  echo "" 
}

json_ok_or_die() {
  local json_text="$1"
  local ok
  ok="$(json_get "$json_text" ok)"
  if [[ "$ok" != "True" && "$ok" != "true" ]]; then
    err="$(json_get "$json_text" error)"
    echo "[ERROR] 请求失败：${err:-unknown}" >&2
    echo "$json_text" >&2
    exit 2
  fi
}

echo "[STEP 0] 预置 3 篇可领取作品（确保 claim 有货）"
TS="$(date -u +%Y%m%d%H%M%S)"
for i in 1 2 3; do
  aid="acc_smoke_${TS}_${i}"
  body=$(printf '{"artwork":{"account_id":"%s","title":"smoke-%s"}}' "$aid" "$aid")
  resp="$(curl_json POST /v1/artworks/submit "$body")"
  json_ok_or_die "$resp"
  echo "  - submitted: $aid"
done

echo "[STEP 1] claim 1 篇作品"
resp_claim="$(curl_json POST /v1/artworks/claim '{"count":1}')"
json_ok_or_die "$resp_claim"
claimed_id="$(json_get "$resp_claim" items)"
# items 是数组，取 items[0].artwork_id（用 python3 拿更稳）
if command -v python3 >/dev/null 2>&1; then
  CLAIMED_ARTWORK_ID="$(python3 - <<'PY'
import json,sys
d=json.loads(sys.stdin.read() or '{}')
items=d.get('items') or []
if items and isinstance(items,list) and isinstance(items[0],dict):
  print(items[0].get('artwork_id','') or '')
else:
  print('')
PY
  <<<"$resp_claim")"
else
  # 兜底：弱解析
  CLAIMED_ARTWORK_ID="$(echo "$resp_claim" | sed -nE 's/.*"artwork_id"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/p' | head -n 1)"
fi

if [[ -z "${CLAIMED_ARTWORK_ID:-}" ]]; then
  echo "[ERROR] claim 响应未解析到 artwork_id" >&2
  echo "$resp_claim" >&2
  exit 2
fi

echo "  - claimed_artwork_id=$CLAIMED_ARTWORK_ID"

echo "[STEP 2] report-damage(full)（触发进入维修区 repair）"
resp_damage="$(curl_json POST /v1/artworks/report-damage "$(printf '{"artwork_id":"%s","kind":"full","note":"smoke"}' "$CLAIMED_ARTWORK_ID")")"
json_ok_or_die "$resp_damage"
REPAIR_TARGET_ID="$(json_get "$resp_damage" replaced_artwork_id)"
if [[ -z "$REPAIR_TARGET_ID" ]]; then
  echo "[ERROR] report-damage 响应未解析到 replaced_artwork_id" >&2
  echo "$resp_damage" >&2
  exit 2
fi

echo "  - repair_target_id=$REPAIR_TARGET_ID"

echo "[STEP 3] 维修者领取该作品（/v1/repairs/claim），直到拿到目标 id"
REP_CLAIMED_ID=""
for attempt in 1 2 3 4 5 6 7 8 9 10; do
  resp_rc="$(curl_json POST /v1/repairs/claim '{"count":1}')"
  ok="$(json_get "$resp_rc" ok)"
  if [[ "$ok" != "True" && "$ok" != "true" ]]; then
    # no_repairable_artwork 等
    echo "  - attempt=$attempt claim_repair failed: $(json_get "$resp_rc" error)"
    continue
  fi

  if command -v python3 >/dev/null 2>&1; then
    REP_CLAIMED_ID="$(python3 - <<'PY'
import json,sys
d=json.loads(sys.stdin.read() or '{}')
items=d.get('items') or []
if items and isinstance(items,list) and isinstance(items[0],dict):
  print(items[0].get('artwork_id','') or '')
else:
  print('')
PY
    <<<"$resp_rc")"
  else
    REP_CLAIMED_ID="$(echo "$resp_rc" | sed -nE 's/.*"artwork_id"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/p' | head -n 1)"
  fi

  echo "  - attempt=$attempt got_repair_artwork_id=$REP_CLAIMED_ID"

  if [[ "$REP_CLAIMED_ID" == "$REPAIR_TARGET_ID" ]]; then
    break
  fi
  REP_CLAIMED_ID=""
done

if [[ "$REP_CLAIMED_ID" != "$REPAIR_TARGET_ID" ]]; then
  echo "[ERROR] 未能在 repairs/claim 中领取到目标作品：$REPAIR_TARGET_ID" >&2
  exit 2
fi

echo "[STEP 4] submit-failed ×3（每次失败后需要再次 claim 回来）"
for round in 1 2 3; do
  resp_fail="$(curl_json POST /v1/repairs/submit-failed "$(printf '{"artwork_id":"%s","note":"smoke-round-%s"}' "$REPAIR_TARGET_ID" "$round")")"
  json_ok_or_die "$resp_fail"
  status="$(json_get "$resp_fail" status)"
  fail_count="$(json_get "$resp_fail" repair_fail_count)"
  echo "  - round=$round status=$status repair_fail_count=$fail_count"

  if [[ "$round" == "3" ]]; then
    if [[ "$status" != "graveyard" ]]; then
      echo "[ERROR] 预期 round=3 后进入 graveyard，但实际 status=$status" >&2
      echo "$resp_fail" >&2
      exit 2
    fi
    break
  fi

  # 继续下一轮前：必须再次领取 repair_claimed（因为 submit-failed 会把状态退回 repair）
  got=""
  for attempt in 1 2 3 4 5 6 7 8 9 10; do
    resp_rc2="$(curl_json POST /v1/repairs/claim '{"count":1}')"
    ok2="$(json_get "$resp_rc2" ok)"
    if [[ "$ok2" != "True" && "$ok2" != "true" ]]; then
      echo "    - re-claim attempt=$attempt failed: $(json_get "$resp_rc2" error)"
      continue
    fi

    if command -v python3 >/dev/null 2>&1; then
      got="$(python3 - <<'PY'
import json,sys
d=json.loads(sys.stdin.read() or '{}')
items=d.get('items') or []
if items and isinstance(items,list) and isinstance(items[0],dict):
  print(items[0].get('artwork_id','') or '')
else:
  print('')
PY
      <<<"$resp_rc2")"
    else
      got="$(echo "$resp_rc2" | sed -nE 's/.*"artwork_id"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/p' | head -n 1)"
    fi

    echo "    - re-claim attempt=$attempt got=$got"
    [[ "$got" == "$REPAIR_TARGET_ID" ]] && break
    got=""
  done

  if [[ "$got" != "$REPAIR_TARGET_ID" ]]; then
    echo "[ERROR] 未能再次领取到目标作品（用于下一轮失败）：$REPAIR_TARGET_ID" >&2
    exit 2
  fi
done

echo "[STEP 5] 再次 submit(同 account_id=artwork_id) 期望被 tombstone 拒绝"
resp_submit2="$(curl_json POST /v1/artworks/submit "$(printf '{"artwork":{"account_id":"%s","title":"smoke-resubmit"}}' "$REPAIR_TARGET_ID")")"

ok2="$(json_get "$resp_submit2" ok)"
err2="$(json_get "$resp_submit2" error)"

if [[ "$ok2" == "True" || "$ok2" == "true" ]]; then
  echo "[ERROR] 预期 re-submit 被拒绝，但返回 ok=true" >&2
  echo "$resp_submit2" >&2
  exit 2
fi

if [[ "$err2" != "tombstoned_artwork_id" ]]; then
  echo "[ERROR] 预期错误 tombstoned_artwork_id，但实际 error=$err2" >&2
  echo "$resp_submit2" >&2
  exit 2
fi

echo "[OK] 冒烟通过：graveyard+tombstone 生效；重复 submit 被拒绝（tombstoned_artwork_id）"
