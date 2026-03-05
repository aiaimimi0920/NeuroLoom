#!/usr/bin/env bash
set -euo pipefail

# 单次续杯（Unix 对齐 Windows 版）
# 探测 -> 上报 -> topup -> 写入 -> 删除失效 -> 增量闭环

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
if [ "$(basename "$SCRIPT_DIR")" = "unix" ]; then
  ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
else
  ROOT_DIR="$SCRIPT_DIR"
fi

LIB_SH="$SCRIPT_DIR/json.sh"
# shellcheck disable=SC1090
source "$LIB_SH"

need_cmd curl
need_json_parser

CFG_ENV="$ROOT_DIR/无限续杯配置.env"
FINAL_REPORT="$ROOT_DIR/out/最终续杯报告.json"
REPLAY_QUEUE_FILE="$ROOT_DIR/out/replay_feedback_queue.txt"
ITER_SCOPE_FILE="$ROOT_DIR/out/refill_iter_scope.txt"
NEXT_SCOPE_FILE="$ROOT_DIR/out/refill_iter_scope_next.txt"

MODE_SYNC_ALL=0
MODE_FROM_TASK=0
MODE_PROBE_WORKER=0
PROBE_FILE=""
PROBE_BASE=""
PROBE_PREFIX=""

if [[ "${1:-}" == "--probe-one-worker" ]]; then
  MODE_PROBE_WORKER=1
  PROBE_FILE="${2:-}"
  PROBE_BASE="${3:-}"
  PROBE_PREFIX="${4:-}"
  WHAM_PROXY_MODE="${5:-auto}"
  WHAM_CONNECT_TIMEOUT="${6:-5}"
  WHAM_MAX_TIME="${7:-15}"
fi

while [[ "$MODE_PROBE_WORKER" == "0" && $# -gt 0 ]]; do
  case "$1" in
    --sync-all)
      MODE_SYNC_ALL=1
      shift
      ;;
    --from-task)
      MODE_FROM_TASK=1
      shift
      ;;
    *)
      break
      ;;
  esac
done

SERVER_URL_ARG="${1:-}"
USER_KEY_ARG="${2:-}"
SERVER_URL=""
USER_KEY=""
ACCOUNTS_DIR="${ACCOUNTS_DIR:-$ROOT_DIR/accounts}"
TARGET_POOL_SIZE="${TARGET_POOL_SIZE:-10}"
TOTAL_HOLD_LIMIT="${TOTAL_HOLD_LIMIT:-50}"
SYNC_TARGET_DIR="${SYNC_TARGET_DIR:-}"
RUN_OUTPUT_MODE="${RUN_OUTPUT_MODE:-compact}"
WHAM_PROXY_MODE="${WHAM_PROXY_MODE:-auto}"
WHAM_CONNECT_TIMEOUT="${WHAM_CONNECT_TIMEOUT:-5}"
WHAM_MAX_TIME="${WHAM_MAX_TIME:-15}"
TOPUP_CONNECT_TIMEOUT="${TOPUP_CONNECT_TIMEOUT:-10}"
TOPUP_MAX_TIME="${TOPUP_MAX_TIME:-180}"
TOPUP_RETRY="${TOPUP_RETRY:-3}"
TOPUP_RETRY_DELAY="${TOPUP_RETRY_DELAY:-3}"
PROBE_PARALLEL="${PROBE_PARALLEL:-6}"
REFILL_ITER_MAX="${REFILL_ITER_MAX:-6}"

sanitize_env_to_tmp() {
  local src="$1"
  local dst="$2"
  local line first_line=1
  : > "$dst"
  while IFS= read -r line || [[ -n "$line" ]]; do
    if [[ "$first_line" == "1" ]]; then
      line="${line#$'\xEF\xBB\xBF'}"
      first_line=0
    fi
    line="${line%$'\r'}"
    printf '%s\n' "$line" >> "$dst"
  done < "$src"
}

if [[ -f "$CFG_ENV" ]]; then
  cfg_tmp="${CFG_ENV}.sanitized.$$"
  sanitize_env_to_tmp "$CFG_ENV" "$cfg_tmp" 2>/dev/null || cp -f "$CFG_ENV" "$cfg_tmp"
  # shellcheck disable=SC1090
  source "$cfg_tmp" || true
  rm -f "$cfg_tmp" >/dev/null 2>&1 || true
fi

trim_cr() {
  local v="${1-}"
  v="${v//$'\r'/}"
  echo "$v"
}

SERVER_URL="${SERVER_URL_ARG:-${SERVER_URL:-}}"
USER_KEY="${USER_KEY_ARG:-${USER_KEY:-}}"

SERVER_URL="$(trim_cr "$SERVER_URL")"
USER_KEY="$(trim_cr "$USER_KEY")"
ACCOUNTS_DIR="$(trim_cr "${ACCOUNTS_DIR:-$ROOT_DIR/accounts}")"
TARGET_POOL_SIZE="$(trim_cr "${TARGET_POOL_SIZE:-10}")"
TOTAL_HOLD_LIMIT="$(trim_cr "${TOTAL_HOLD_LIMIT:-50}")"
SYNC_TARGET_DIR="$(trim_cr "${SYNC_TARGET_DIR:-}")"
RUN_OUTPUT_MODE="$(trim_cr "${RUN_OUTPUT_MODE:-compact}")"
WHAM_PROXY_MODE="$(trim_cr "${WHAM_PROXY_MODE:-auto}")"
WHAM_CONNECT_TIMEOUT="$(trim_cr "${WHAM_CONNECT_TIMEOUT:-5}")"
WHAM_MAX_TIME="$(trim_cr "${WHAM_MAX_TIME:-15}")"
TOPUP_CONNECT_TIMEOUT="$(trim_cr "${TOPUP_CONNECT_TIMEOUT:-10}")"
TOPUP_MAX_TIME="$(trim_cr "${TOPUP_MAX_TIME:-180}")"
TOPUP_RETRY="$(trim_cr "${TOPUP_RETRY:-3}")"
TOPUP_RETRY_DELAY="$(trim_cr "${TOPUP_RETRY_DELAY:-3}")"
PROBE_PARALLEL="$(trim_cr "${PROBE_PARALLEL:-6}")"
REFILL_ITER_MAX="$(trim_cr "${REFILL_ITER_MAX:-6}")"

if [[ -z "$ACCOUNTS_DIR" ]]; then
  ACCOUNTS_DIR="$ROOT_DIR/accounts"
fi

normalize_int() {
  local v="$1"
  local d="$2"
  local min="$3"
  local max="$4"
  if ! [[ "$v" =~ ^[0-9]+$ ]]; then
    v="$d"
  fi
  if (( v < min )); then
    v="$d"
  fi
  if (( max > 0 && v > max )); then
    v="$max"
  fi
  echo "$v"
}

TARGET_POOL_SIZE="$(normalize_int "$TARGET_POOL_SIZE" 10 1 500)"
TOTAL_HOLD_LIMIT="$(normalize_int "$TOTAL_HOLD_LIMIT" 50 1 10000)"
WHAM_CONNECT_TIMEOUT="$(normalize_int "$WHAM_CONNECT_TIMEOUT" 5 1 120)"
WHAM_MAX_TIME="$(normalize_int "$WHAM_MAX_TIME" 15 5 300)"
TOPUP_CONNECT_TIMEOUT="$(normalize_int "$TOPUP_CONNECT_TIMEOUT" 10 1 120)"
TOPUP_MAX_TIME="$(normalize_int "$TOPUP_MAX_TIME" 180 30 1800)"
TOPUP_RETRY="$(normalize_int "$TOPUP_RETRY" 3 0 10)"
TOPUP_RETRY_DELAY="$(normalize_int "$TOPUP_RETRY_DELAY" 3 1 30)"
PROBE_PARALLEL="$(normalize_int "$PROBE_PARALLEL" 6 1 32)"
REFILL_ITER_MAX="$(normalize_int "$REFILL_ITER_MAX" 6 1 20)"

normalize_proxy_mode() {
  local m
  m="$(printf '%s' "${1:-auto}" | tr '[:upper:]' '[:lower:]')"
  if [[ "$m" != "auto" && "$m" != "direct" ]]; then
    m="auto"
  fi
  echo "$m"
}
WHAM_PROXY_MODE="$(normalize_proxy_mode "$WHAM_PROXY_MODE")"

if [[ "$MODE_PROBE_WORKER" == "0" ]]; then
  if [[ -z "$SERVER_URL" || -z "$USER_KEY" ]]; then
    echo "[ERROR] 未配置 SERVER_URL/USER_KEY，请先运行：bash \"$SCRIPT_DIR/无限续杯.sh\""
    exit 2
  fi
fi

mkdir -p "$ACCOUNTS_DIR" "$ROOT_DIR/out"
if [[ ! -d "$ACCOUNTS_DIR" ]]; then
  echo "[ERROR] 账户目录不存在且创建失败：$ACCOUNTS_DIR"
  exit 3
fi

if [[ ! -f "$REPLAY_QUEUE_FILE" ]]; then
  : > "$REPLAY_QUEUE_FILE"
fi

cleanup_old_out() {
  [[ -d "$ROOT_DIR/out" ]] || return 0
  mapfile -t dirs < <(find "$ROOT_DIR/out" -mindepth 1 -maxdepth 1 -type d ! -name latest ! -name latest-syncall -print 2>/dev/null | while read -r d; do
    stat_ts="$(stat -c %Y "$d" 2>/dev/null || stat -f %m "$d" 2>/dev/null || echo 0)"
    printf '%s|%s\n' "$stat_ts" "$d"
  done | sort -t'|' -k1,1nr | cut -d'|' -f2-)
  local i=0
  for d in "${dirs[@]:-}"; do
    i=$((i + 1))
    if (( i > 10 )); then
      rm -rf "$d" >/dev/null 2>&1 || true
    fi
  done
}

write_final_report() {
  local mode="$1"
  local status="$2"
  local total="$3"
  local probed_ok="$4"
  local net_fail="$5"
  local invalid="$6"
  local out_dir="$7"
  local generated_at
  generated_at="$(date '+%Y-%m-%dT%H:%M:%S%z')"

  if command -v jq >/dev/null 2>&1; then
    jq -n \
      --arg generated_at "$generated_at" \
      --arg mode "$mode" \
      --arg status "$status" \
      --arg out_dir "$out_dir" \
      --arg final_report "$FINAL_REPORT" \
      --argjson total "$total" \
      --argjson probed_ok "$probed_ok" \
      --argjson net_fail "$net_fail" \
      --argjson invalid "$invalid" \
      '{
        generated_at: $generated_at,
        mode: $mode,
        status: $status,
        total: $total,
        probed_ok: $probed_ok,
        net_fail: $net_fail,
        invalid_401_429: $invalid,
        out_dir: $out_dir,
        final_report: $final_report
      }' > "$FINAL_REPORT" 2>/dev/null || true
    return 0
  fi

  if [[ "$(uname 2>/dev/null || echo unknown)" == "Darwin" ]] && command -v osascript >/dev/null 2>&1; then
    FINAL_REPORT_PATH="$FINAL_REPORT" \
    REPORT_GENERATED_AT="$generated_at" \
    REPORT_MODE="$mode" \
    REPORT_STATUS="$status" \
    REPORT_TOTAL="$total" \
    REPORT_PROBED_OK="$probed_ok" \
    REPORT_NET_FAIL="$net_fail" \
    REPORT_INVALID="$invalid" \
    REPORT_OUT_DIR="$out_dir" \
    osascript -l JavaScript <<'OSA' >/dev/null 2>&1 || true
ObjC.import('Foundation');
ObjC.import('stdlib');
const path = $.getenv('FINAL_REPORT_PATH');
const obj = {
  generated_at: $.getenv('REPORT_GENERATED_AT') || '',
  mode: $.getenv('REPORT_MODE') || '',
  status: $.getenv('REPORT_STATUS') || '',
  total: Number($.getenv('REPORT_TOTAL') || '0') || 0,
  probed_ok: Number($.getenv('REPORT_PROBED_OK') || '0') || 0,
  net_fail: Number($.getenv('REPORT_NET_FAIL') || '0') || 0,
  invalid_401_429: Number($.getenv('REPORT_INVALID') || '0') || 0,
  out_dir: $.getenv('REPORT_OUT_DIR') || '',
  final_report: path
};
const text = JSON.stringify(obj, null, 2);
$(text).writeToFileAtomicallyEncodingError($(path), true, $.NSUTF8StringEncoding, null);
OSA
    return 0
  fi

  python3 - "$FINAL_REPORT" "$mode" "$status" "$total" "$probed_ok" "$net_fail" "$invalid" "$out_dir" <<'PY' 2>/dev/null || true
import json, sys, datetime
p, mode, status, total, probed, netf, invalid, outdir = sys.argv[1:9]
obj = {
  "generated_at": datetime.datetime.now().astimezone().isoformat(timespec="seconds"),
  "mode": mode,
  "status": status,
  "total": int(total),
  "probed_ok": int(probed),
  "net_fail": int(netf),
  "invalid_401_429": int(invalid),
  "out_dir": outdir,
  "final_report": p,
}
with open(p, "w", encoding="utf-8") as f:
  json.dump(obj, f, ensure_ascii=False, indent=2)
PY
}

upsert_env_key() {
  local key="$1"
  local value="$2"
  [[ -f "$CFG_ENV" ]] || return 0

  if grep -q "^${key}=" "$CFG_ENV" 2>/dev/null; then
    awk -v k="$key" -v v="$value" 'BEGIN{FS=OFS="="}
      $1==k {$0=k"="v}
      {print}
    ' "$CFG_ENV" > "$CFG_ENV.tmp" 2>/dev/null && mv -f "$CFG_ENV.tmp" "$CFG_ENV"
  else
    printf '%s=%s\n' "$key" "$value" >> "$CFG_ENV"
  fi
}

sync_managed_json() {
  local target accounts linked=0 removed=0
  target="${SYNC_TARGET_DIR:-}"
  accounts="$ACCOUNTS_DIR"
  local manifest
  manifest="$target/.infinite_refill_sync_manifest.txt"

  [[ -z "$target" ]] && return 0

  mkdir -p "$target" "$accounts"
  shopt -s nullglob

  local files=("$accounts"/无限续杯-*.json)
  if [[ ${#files[@]} -eq 0 ]]; then
    files=("$accounts"/*.json)
  fi

  local names=() f base tp
  for f in "${files[@]}"; do
    [[ -e "$f" ]] || continue
    base="$(basename "$f")"
    names+=("$base")
  done

  if [[ -f "$manifest" ]]; then
    while IFS= read -r base; do
      [[ -n "$base" ]] || continue
      tp="$target/$base"
      if [[ ! -e "$accounts/$base" && -L "$tp" ]]; then
        rm -f "$tp" 2>/dev/null || true
        removed=$((removed + 1))
      fi
    done < "$manifest"
  fi

  for f in "${files[@]}"; do
    [[ -e "$f" ]] || continue
    base="$(basename "$f")"
    tp="$target/$base"
    if [[ -L "$tp" ]]; then
      rm -f "$tp" 2>/dev/null || true
    elif [[ -e "$tp" ]]; then
      continue
    fi
    ln -s "$f" "$tp" 2>/dev/null || true
    [[ -L "$tp" ]] && linked=$((linked + 1))
  done

  printf "%s\n" "${names[@]}" > "$manifest"
  echo "[OK] 已确保同步软链接：${target}（linked=${linked} removed=${removed}）"
}

consume_replay_queue_by_reports() {
  local report_jsonl="$1"
  local parser sent_tmp queue_tmp out_tmp
  parser="$(detect_json_parser)"
  sent_tmp="${report_jsonl}.sent"
  queue_tmp="${REPLAY_QUEUE_FILE}.q"
  out_tmp="${REPLAY_QUEUE_FILE}.tmp"

  case "$parser" in
    jq)
      jq -r 'fromjson? | .file_name // empty' "$report_jsonl" 2>/dev/null | awk 'NF>0{print}' > "$sent_tmp" || :
      ;;
    osascript)
      osascript -l JavaScript <<OSA > "$sent_tmp" 2>/dev/null || :
ObjC.import('Foundation');
function readText(path){
  try { return $.NSString.stringWithContentsOfFileEncodingError(path, $.NSUTF8StringEncoding, null).js; }
  catch(e) { return ''; }
}
const lines = readText('$report_jsonl').split(/\r?\n/);
for (const ln of lines) {
  if (!ln) continue;
  try {
    const o = JSON.parse(ln);
    const n = String(o.file_name || '').trim();
    if (n) console.log(n);
  } catch(e) {}
}
OSA
      ;;
    *)
      python3 - "$REPLAY_QUEUE_FILE" "$report_jsonl" <<'PY' >/dev/null 2>&1 || true
import json, sys, os
qf, rep = sys.argv[1:3]
queue = []
if os.path.exists(qf):
  with open(qf, 'r', encoding='utf-8') as f:
    queue = [x.strip() for x in f if x.strip()]
sent = set()
if os.path.exists(rep):
  with open(rep, 'r', encoding='utf-8') as f:
    for ln in f:
      ln = ln.strip()
      if not ln:
        continue
      try:
        o = json.loads(ln)
      except Exception:
        continue
      n = str(o.get('file_name') or '').strip()
      if n:
        sent.add(n)
new_q = [x for x in queue if x not in sent]
with open(qf, 'w', encoding='utf-8') as f:
  for n in dict.fromkeys(new_q):
    f.write(n + '\n')
PY
      return 0
      ;;
  esac

  [[ -f "$REPLAY_QUEUE_FILE" ]] || : > "$REPLAY_QUEUE_FILE"
  awk 'NF>0 && !seen[$0]++{print}' "$REPLAY_QUEUE_FILE" > "$queue_tmp" 2>/dev/null || : > "$queue_tmp"
  awk 'NF>0 && !seen[$0]++{print}' "$sent_tmp" > "${sent_tmp}.u" 2>/dev/null || : > "${sent_tmp}.u"
  awk 'NR==FNR{del[$0]=1;next} NF>0 && !del[$0]{print}' "${sent_tmp}.u" "$queue_tmp" > "$out_tmp" 2>/dev/null || : > "$out_tmp"
  mv -f "$out_tmp" "$REPLAY_QUEUE_FILE" 2>/dev/null || true
  rm -f "$sent_tmp" "${sent_tmp}.u" "$queue_tmp" >/dev/null 2>&1 || true
}

build_response_scope() {
  local resp_json="$1"
  local scope_file="$2"
  local parser
  parser="$(detect_json_parser)"

  case "$parser" in
    jq)
      jq -r '.accounts[]? | (.file_name // (if (.account_id // "") != "" then ("codex-" + (.account_id|tostring) + ".json") else empty end))' \
        "$resp_json" 2>/dev/null \
        | awk -v acc="$ACCOUNTS_DIR" 'NF>0 && !seen[$0]++ { cmd = "test -f \"" acc "/" $0 "\""; if (system(cmd) == 0) print $0; }' \
        > "$scope_file" || true
      ;;
    osascript)
      osascript -l JavaScript <<OSA > "$scope_file" 2>/dev/null || true
ObjC.import('Foundation');
ObjC.import('stdlib');
function readText(path){
  try { return $.NSString.stringWithContentsOfFileEncodingError(path, $.NSUTF8StringEncoding, null).js; }
  catch(e) { return ''; }
}
const fs = $.NSFileManager.defaultManager;
let obj = {};
try { obj = JSON.parse(readText('$resp_json')); } catch(e) { obj = {}; }
const arr = Array.isArray(obj.accounts) ? obj.accounts : [];
const seen = {};
for (const a of arr) {
  let fn = String(a.file_name || '').trim();
  if (!fn) {
    const aid = String(a.account_id || '').trim();
    if (aid) fn = 'codex-' + aid + '.json';
  }
  if (!fn || seen[fn]) continue;
  const p = '$ACCOUNTS_DIR/' + fn;
  if (fs.fileExistsAtPath(p)) {
    seen[fn] = 1;
    console.log(fn);
  }
}
OSA
      ;;
    *)
      python3 - "$resp_json" "$ACCOUNTS_DIR" "$scope_file" <<'PY' >/dev/null 2>&1 || true
import json, sys, os
resp, acc, out = sys.argv[1:4]
names = []
try:
  with open(resp, 'r', encoding='utf-8') as f:
    obj = json.load(f)
  for a in (obj.get('accounts') or []):
    fn = str(a.get('file_name') or '').strip()
    if not fn:
      aid = str(a.get('account_id') or '').strip()
      if aid:
        fn = f'codex-{aid}.json'
    if fn and os.path.exists(os.path.join(acc, fn)):
      names.append(fn)
except Exception:
  pass
if names:
  with open(out, 'w', encoding='utf-8') as f:
    for n in dict.fromkeys(names):
      f.write(n + '\n')
else:
  try:
    os.remove(out)
  except Exception:
    pass
PY
      return 0
      ;;
  esac

  if [[ ! -s "$scope_file" ]]; then
    rm -f "$scope_file" >/dev/null 2>&1 || true
  fi
}

parse_topup_response() {
  local resp_json="$1"
  local accounts_dir="$2"
  local replay_queue="$3"
  local dl_retry="${4:-3}"
  local dl_timeout="${5:-30}"
  local parser
  parser="$(detect_json_parser)"

  if [[ "$parser" == "jq" ]]; then
    local ok server_count written failed total_hold_limit confidence_replay_percent issued_replay_count auto_disabled abuse_auto_banned
    local replay_tmp old_tmp merged_tmp
    ok="$(jq -r '.ok // false' "$resp_json" 2>/dev/null || echo false)"
    if [[ "$ok" != "true" ]]; then
      echo "ERROR=$(jq -r '.error // "topup_failed"' "$resp_json" 2>/dev/null || echo topup_failed)"
      return 2
    fi

    mkdir -p "$accounts_dir"
    server_count="$(jq -r '(.accounts // []) | length' "$resp_json" 2>/dev/null || echo 0)"
    written=0
    failed=0
    replay_tmp="${replay_queue}.new"
    : > "$replay_tmp"

    while IFS= read -r acc; do
      [[ -n "$acc" ]] || continue
      local aid fn dst has_auth dl auth_tmp dl_ok
      aid="$(printf '%s' "$acc" | jq -r '.account_id // ""' 2>/dev/null)"
      fn="$(printf '%s' "$acc" | jq -r '.file_name // ""' 2>/dev/null)"
      if [[ -z "$fn" && -n "$aid" ]]; then
        fn="codex-$aid.json"
      fi
      [[ -n "$fn" ]] || continue
      dst="$accounts_dir/$fn"
      [[ -f "$dst" ]] && continue

      has_auth="$(printf '%s' "$acc" | jq -r 'has("auth_json") and (.auth_json != null)' 2>/dev/null || echo false)"
      dl_ok=0
      if [[ "$has_auth" == "true" ]]; then
        if printf '%s' "$acc" | jq -c '.auth_json' > "$dst" 2>/dev/null; then
          printf '\n' >> "$dst"
          dl_ok=1
        fi
      else
        dl="$(printf '%s' "$acc" | jq -r '.download_url // ""' 2>/dev/null)"
        if [[ -n "$dl" ]]; then
          auth_tmp="${dst}.tmp"
          if curl -sS -L --connect-timeout "$dl_timeout" --max-time "$dl_timeout" --retry "$dl_retry" --retry-all-errors --retry-delay "$TOPUP_RETRY_DELAY" -A "curl/7.0" "$dl" -o "$auth_tmp" 2>/dev/null; then
            if jq -e . "$auth_tmp" >/dev/null 2>&1; then
              mv -f "$auth_tmp" "$dst"
              printf '\n' >> "$dst"
              dl_ok=1
            else
              rm -f "$auth_tmp" >/dev/null 2>&1 || true
            fi
          fi
        fi
      fi

      if [[ "$dl_ok" == "1" ]]; then
        written=$((written + 1))
        if [[ "$(printf '%s' "$acc" | jq -r '.replay_from_confidence == true' 2>/dev/null || echo false)" == "true" ]]; then
          printf '%s\n' "$fn" >> "$replay_tmp"
        fi
      else
        failed=$((failed + 1))
        dl="$(printf '%s' "$acc" | jq -r '.download_url // ""' 2>/dev/null)"
        echo "DL_FAIL=${aid}|${dl}|download_failed"
      fi
    done < <(jq -c '.accounts[]?' "$resp_json" 2>/dev/null)

    old_tmp="${replay_queue}.old"
    merged_tmp="${replay_queue}.tmp"
    [[ -f "$replay_queue" ]] && awk 'NF>0{print}' "$replay_queue" > "$old_tmp" || : > "$old_tmp"
    cat "$old_tmp" "$replay_tmp" 2>/dev/null | awk 'NF>0 && !seen[$0]++{print}' > "$merged_tmp"
    mv -f "$merged_tmp" "$replay_queue" >/dev/null 2>&1 || true
    rm -f "$replay_tmp" "$old_tmp" >/dev/null 2>&1 || true

    total_hold_limit="$(jq -r 'if .total_hold_limit == null then "" else (.total_hold_limit|tostring) end' "$resp_json" 2>/dev/null || true)"
    confidence_replay_percent="$(jq -r 'if .confidence_replay_percent == null then "" else (.confidence_replay_percent|tostring) end' "$resp_json" 2>/dev/null || true)"
    issued_replay_count="$(jq -r 'if .issued_replay_count == null then "" else (.issued_replay_count|tostring) end' "$resp_json" 2>/dev/null || true)"
    auto_disabled="$(jq -r 'if .auto_disabled == null then "" else (.auto_disabled|tostring) end' "$resp_json" 2>/dev/null || true)"
    abuse_auto_banned="$(jq -r 'if .abuse_auto_banned == null then "" else (.abuse_auto_banned|tostring) end' "$resp_json" 2>/dev/null || true)"

    echo "SERVER_COUNT=$server_count"
    echo "WRITTEN=$written"
    echo "WRITE_FAILED=$failed"
    [[ -n "$total_hold_limit" ]] && echo "TOTAL_HOLD_LIMIT=$total_hold_limit"
    [[ -n "$confidence_replay_percent" ]] && echo "CONFIDENCE_REPLAY_PERCENT=$confidence_replay_percent"
    [[ -n "$issued_replay_count" ]] && echo "ISSUED_REPLAY_COUNT=$issued_replay_count"
    [[ -n "$auto_disabled" ]] && echo "AUTO_DISABLED=$auto_disabled"
    [[ -n "$abuse_auto_banned" ]] && echo "ABUSE_AUTO_BANNED=$abuse_auto_banned"
    return 0
  fi

  if [[ "$parser" == "osascript" ]]; then
    RESP_JSON="$resp_json" \
    ACCOUNTS_DIR="$accounts_dir" \
    REPLAY_QUEUE="$replay_queue" \
    DL_TIMEOUT="$dl_timeout" \
    DL_RETRY="$dl_retry" \
    RETRY_DELAY="$TOPUP_RETRY_DELAY" \
    osascript -l JavaScript <<'OSA' 2>/dev/null || true
ObjC.import('Foundation');
ObjC.import('stdlib');
function readText(path){
  try { return $.NSString.stringWithContentsOfFileEncodingError(path, $.NSUTF8StringEncoding, null).js; }
  catch(e){ return ''; }
}
const respPath = $.getenv('RESP_JSON');
const accDir = $.getenv('ACCOUNTS_DIR');
const qf = $.getenv('REPLAY_QUEUE');
const dlTimeout = $.getenv('DL_TIMEOUT');
const dlRetry = $.getenv('DL_RETRY');
const retryDelay = $.getenv('RETRY_DELAY');
let obj = {};
try { obj = JSON.parse(readText(respPath)); } catch(e) { console.log('ERROR=bad response json'); $.exit(0); }
if (!obj.ok) { console.log('ERROR=' + String(obj.error || 'topup_failed')); $.exit(0); }
const fm = $.NSFileManager.defaultManager;
$.system('/bin/mkdir -p "' + String(accDir || '').replace(/"/g, '\\"') + '"');
let written = 0, failed = 0;
let replayNames = [];
const accounts = Array.isArray(obj.accounts) ? obj.accounts : [];
for (const a of accounts) {
  const aid = String(a.account_id || '').trim();
  let fn = String(a.file_name || '').trim();
  if (!fn && aid) fn = 'codex-' + aid + '.json';
  if (!fn) continue;
  const dst = accDir + '/' + fn;
  if (fm.fileExistsAtPath(dst)) continue;
  let ok = false;
  if (a.auth_json !== undefined && a.auth_json !== null) {
    try {
      const text = JSON.stringify(a.auth_json);
      if (text && text !== 'undefined') {
        $(text + '\n').writeToFileAtomicallyEncodingError($(dst), true, $.NSUTF8StringEncoding, null);
        ok = fm.fileExistsAtPath(dst);
      }
    } catch(e) {}
  } else {
    const dl = String(a.download_url || '').trim();
    if (dl) {
      const cmd = '/usr/bin/curl -sS -L --connect-timeout ' + dlTimeout + ' --max-time ' + dlTimeout + ' --retry ' + dlRetry + ' --retry-all-errors --retry-delay ' + retryDelay + ' -A "curl/7.0" "' + dl.replace(/"/g, '\\"') + '" -o "' + dst.replace(/"/g, '\\"') + '"';
      try { $.system(cmd); ok = fm.fileExistsAtPath(dst); } catch(e) { ok = false; }
    }
  }
  if (ok) {
    written++;
    if (a.replay_from_confidence === true) {
      replayNames.push(fn);
    }
  } else {
    failed++;
    console.log('DL_FAIL=' + aid + '|' + String(a.download_url || '') + '|download_failed');
  }
}
let old = [];
try { old = readText(qf).split(/\r?\n/).filter(Boolean); } catch(e) {}
const seen = {};
const merged = [];
for (const n of old.concat(replayNames)) { if (n && !seen[n]) { seen[n]=1; merged.push(n);} }
try {
  const ns = $(merged.join('\n') + (merged.length ? '\n' : ''));
  ns.writeToFileAtomicallyEncodingError($(qf), true, $.NSUTF8StringEncoding, null);
} catch(e) {}
console.log('SERVER_COUNT=' + accounts.length);
console.log('WRITTEN=' + written);
console.log('WRITE_FAILED=' + failed);
if (obj.total_hold_limit !== undefined && obj.total_hold_limit !== null) console.log('TOTAL_HOLD_LIMIT=' + String(obj.total_hold_limit));
if (obj.confidence_replay_percent !== undefined && obj.confidence_replay_percent !== null) console.log('CONFIDENCE_REPLAY_PERCENT=' + String(obj.confidence_replay_percent));
if (obj.issued_replay_count !== undefined && obj.issued_replay_count !== null) console.log('ISSUED_REPLAY_COUNT=' + String(obj.issued_replay_count));
if (obj.auto_disabled !== undefined && obj.auto_disabled !== null) console.log('AUTO_DISABLED=' + String(obj.auto_disabled));
if (obj.abuse_auto_banned !== undefined && obj.abuse_auto_banned !== null) console.log('ABUSE_AUTO_BANNED=' + String(obj.abuse_auto_banned));
OSA
    return 0
  fi

  python3 - "$resp_json" "$accounts_dir" "$replay_queue" "$dl_timeout" "$dl_retry" "$TOPUP_RETRY_DELAY" <<'PY'
import json, os, sys, time
from urllib.request import Request, urlopen

resp_file, acc_dir, qf, timeout_s, retry_n, retry_delay = sys.argv[1:7]
timeout_s = int(timeout_s)
retry_n = int(retry_n)
retry_delay = int(retry_delay)

def dl_json(url, timeout, retry, delay):
  last = None
  for i in range(retry + 1):
    try:
      req = Request(url, headers={'User-Agent': 'curl/7.0'})
      with urlopen(req, timeout=timeout) as r:
        data = r.read().decode('utf-8', errors='replace')
      obj = json.loads(data)
      return obj, None
    except Exception as e:
      last = e
      if i < retry:
        time.sleep(delay)
  return None, last

try:
  with open(resp_file, 'r', encoding='utf-8') as f:
    obj = json.load(f)
except Exception as e:
  print(f'ERROR=bad response json: {e}')
  raise SystemExit(2)

if not obj.get('ok'):
  err = obj.get('error')
  print(f'ERROR={err}')
  raise SystemExit(2)

accs = obj.get('accounts') or []
os.makedirs(acc_dir, exist_ok=True)

server_count = len(accs)
written = 0
failed = 0
replay_names = []

for a in accs:
  aid = str(a.get('account_id') or '').strip()
  fn = str(a.get('file_name') or '').strip()
  if not fn:
    if aid:
      fn = f'codex-{aid}.json'
    else:
      continue
  dst = os.path.join(acc_dir, fn)
  if os.path.exists(dst):
    continue

  payload = None
  if a.get('auth_json') is not None:
    payload = a.get('auth_json')
  else:
    dl = str(a.get('download_url') or '').strip()
    if dl:
      payload, err = dl_json(dl, timeout_s, retry_n, retry_delay)
      if payload is None:
        failed += 1
        print(f'DL_FAIL={aid}|{dl}|{err}')
        continue
    else:
      failed += 1
      continue

  try:
    with open(dst, 'w', encoding='utf-8') as f:
      json.dump(payload, f, ensure_ascii=False, separators=(',', ':'))
      f.write('\n')
    written += 1
    if a.get('replay_from_confidence') is True:
      replay_names.append(fn)
  except Exception:
    failed += 1

old = []
if os.path.exists(qf):
  with open(qf, 'r', encoding='utf-8') as f:
    old = [x.strip() for x in f if x.strip()]
merged = []
seen = set()
for n in old + replay_names:
  if n and n not in seen:
    merged.append(n)
    seen.add(n)
with open(qf, 'w', encoding='utf-8') as f:
  for n in merged:
    f.write(n + '\n')

print(f'SERVER_COUNT={server_count}')
print(f'WRITTEN={written}')
print(f'WRITE_FAILED={failed}')
if obj.get('total_hold_limit') is not None:
  print(f'TOTAL_HOLD_LIMIT={obj.get("total_hold_limit")}')
if obj.get('confidence_replay_percent') is not None:
  print(f'CONFIDENCE_REPLAY_PERCENT={obj.get("confidence_replay_percent")}')
if obj.get('issued_replay_count') is not None:
  print(f'ISSUED_REPLAY_COUNT={obj.get("issued_replay_count")}')
if obj.get('auto_disabled') is not None:
  print(f'AUTO_DISABLED={obj.get("auto_disabled")}')
if obj.get('abuse_auto_banned') is not None:
  print(f'ABUSE_AUTO_BANNED={obj.get("abuse_auto_banned")}')
PY
}

rescue_missing_from_response() {
  local resp_json="$1"
  local accounts_dir="$2"
  local retry_times="$3"
  local timeout_secs="$4"
  local parser
  parser="$(detect_json_parser)"

  if [[ "$parser" == "jq" ]]; then
    local recovered failed
    recovered=0
    failed=0
    mkdir -p "$accounts_dir"

    while IFS= read -r acc; do
      [[ -n "$acc" ]] || continue
      local aid fn dst dl ok i tmp
      aid="$(printf '%s' "$acc" | jq -r '.account_id // ""' 2>/dev/null)"
      fn="$(printf '%s' "$acc" | jq -r '.file_name // ""' 2>/dev/null)"
      [[ -z "$fn" && -n "$aid" ]] && fn="codex-$aid.json"
      [[ -n "$fn" ]] || continue
      dst="$accounts_dir/$fn"
      [[ -f "$dst" ]] && continue
      dl="$(printf '%s' "$acc" | jq -r '.download_url // ""' 2>/dev/null)"
      if [[ -z "$dl" ]]; then
        failed=$((failed + 1))
        continue
      fi
      ok=0
      i=0
      while (( i < retry_times )); do
        i=$((i + 1))
        tmp="${dst}.tmp"
        if curl -sS -L -A "curl/7.0" --connect-timeout "$timeout_secs" --max-time "$timeout_secs" "$dl" -o "$tmp" 2>/dev/null; then
          if jq -e . "$tmp" >/dev/null 2>&1; then
            mv -f "$tmp" "$dst"
            printf '\n' >> "$dst"
            ok=1
            break
          fi
        fi
        rm -f "$tmp" >/dev/null 2>&1 || true
        (( i < retry_times )) && sleep "$TOPUP_RETRY_DELAY"
      done
      if [[ "$ok" == "1" ]]; then
        recovered=$((recovered + 1))
      else
        failed=$((failed + 1))
      fi
    done < <(jq -c '.accounts[]?' "$resp_json" 2>/dev/null)

    echo "$recovered|$failed"
    return 0
  fi

  if [[ "$parser" == "osascript" ]]; then
    osascript -l JavaScript <<OSA 2>/dev/null || echo "0|0"
ObjC.import('Foundation');
ObjC.import('stdlib');
function readText(path){
  try { return $.NSString.stringWithContentsOfFileEncodingError(path, $.NSUTF8StringEncoding, null).js; }
  catch(e){ return ''; }
}
const respPath = '$resp_json';
const accDir = '$accounts_dir';
const retryTimes = Number('$retry_times') || 0;
const timeoutSecs = '$timeout_secs';
const delaySecs = '$TOPUP_RETRY_DELAY';
let obj = {};
try { obj = JSON.parse(readText(respPath)); } catch(e) { console.log('0|0'); $.exit(0); }
const fm = $.NSFileManager.defaultManager;
$.system('/bin/mkdir -p "' + String(accDir || '').replace(/"/g, '\\"') + '"');
let recovered = 0, failed = 0;
const accs = Array.isArray(obj.accounts) ? obj.accounts : [];
for (const a of accs) {
  const aid = String(a.account_id || '').trim();
  let fn = String(a.file_name || '').trim();
  if (!fn && aid) fn = 'codex-' + aid + '.json';
  if (!fn) continue;
  const dst = accDir + '/' + fn;
  if (fm.fileExistsAtPath(dst)) continue;
  const dl = String(a.download_url || '').trim();
  if (!dl) { failed++; continue; }
  let ok = false;
  for (let i = 0; i < retryTimes; i++) {
    const cmd = '/usr/bin/curl -sS -L -A "curl/7.0" --connect-timeout ' + timeoutSecs + ' --max-time ' + timeoutSecs + ' "' + dl.replace(/"/g, '\\"') + '" -o "' + dst.replace(/"/g, '\\"') + '"';
    try { $.system(cmd); ok = fm.fileExistsAtPath(dst); } catch(e) { ok = false; }
    if (ok) break;
    if (i < retryTimes - 1) $.system('/bin/sleep ' + delaySecs);
  }
  if (ok) recovered++; else failed++;
}
console.log(String(recovered) + '|' + String(failed));
OSA
    return 0
  fi

  python3 - "$resp_json" "$accounts_dir" "$retry_times" "$timeout_secs" "$TOPUP_RETRY_DELAY" <<'PY'
import json, os, sys, time
from urllib.request import Request, urlopen

resp, acc, retry_n, timeout_s, delay_s = sys.argv[1:6]
retry_n = int(retry_n)
timeout_s = int(timeout_s)
delay_s = int(delay_s)

try:
  with open(resp, 'r', encoding='utf-8') as f:
    obj = json.load(f)
except Exception:
  print('0|0')
  raise SystemExit(0)

accs = obj.get('accounts') or []
recovered = 0
failed = 0

for a in accs:
  aid = str(a.get('account_id') or '').strip()
  fn = str(a.get('file_name') or '').strip()
  if not fn and aid:
    fn = f'codex-{aid}.json'
  if not fn:
    continue
  dst = os.path.join(acc, fn)
  if os.path.exists(dst):
    continue
  dl = str(a.get('download_url') or '').strip()
  if not dl:
    failed += 1
    continue

  ok = False
  for i in range(retry_n):
    try:
      req = Request(dl, headers={'User-Agent': 'curl/7.0'})
      with urlopen(req, timeout=timeout_s) as r:
        raw = r.read().decode('utf-8', errors='replace')
      payload = json.loads(raw)
      with open(dst, 'w', encoding='utf-8') as f:
        json.dump(payload, f, ensure_ascii=False, separators=(',', ':'))
        f.write('\n')
      recovered += 1
      ok = True
      break
    except Exception:
      if i < retry_n - 1:
        time.sleep(delay_s)
  if not ok:
    failed += 1

print(f'{recovered}|{failed}')
PY
}

cleanup_old_codex_after_sync() {
  local before_list="$1"
  local resp_json="$2"
  local accounts_dir="$3"
  local parser keep_tmp deleted
  parser="$(detect_json_parser)"
  keep_tmp="${before_list}.keep"
  deleted=0

  case "$parser" in
    jq)
      jq -r '.accounts[]? | .account_id // empty | "codex-" + tostring + ".json"' "$resp_json" 2>/dev/null \
        | awk 'NF>0 && !seen[$0]++{print}' > "$keep_tmp" || : > "$keep_tmp"
      ;;
    osascript)
      osascript -l JavaScript <<OSA > "$keep_tmp" 2>/dev/null || : > "$keep_tmp"
ObjC.import('Foundation');
function readText(path){
  try { return $.NSString.stringWithContentsOfFileEncodingError(path, $.NSUTF8StringEncoding, null).js; }
  catch(e) { return ''; }
}
let obj = {};
try { obj = JSON.parse(readText('$resp_json')); } catch(e) { obj = {}; }
const arr = Array.isArray(obj.accounts) ? obj.accounts : [];
const seen = {};
for (const a of arr) {
  const aid = String(a.account_id || '').trim();
  if (!aid) continue;
  const n = 'codex-' + aid + '.json';
  if (!seen[n]) { seen[n] = 1; console.log(n); }
}
OSA
      ;;
    *)
      python3 - "$before_list" "$resp_json" "$accounts_dir" <<'PY'
import os, sys, json
before, resp, acc = sys.argv[1:4]
keep = set()
try:
  with open(resp, 'r', encoding='utf-8') as f:
    obj = json.load(f)
  for a in obj.get('accounts') or []:
    aid = str(a.get('account_id') or '').strip()
    if aid:
      keep.add(f'codex-{aid}.json')
except Exception:
  pass

deleted = 0
if os.path.exists(before):
  with open(before, 'r', encoding='utf-8') as f:
    for ln in f:
      n = ln.strip()
      if not n or not n.startswith('codex-') or not n.endswith('.json'):
        continue
      if n in keep:
        continue
      p = os.path.join(acc, n)
      if os.path.exists(p):
        try:
          os.remove(p)
          if not os.path.exists(p):
            deleted += 1
        except Exception:
          pass
print(deleted)
PY
      return 0
      ;;
  esac

  [[ -f "$before_list" ]] || {
    echo 0
    rm -f "$keep_tmp" >/dev/null 2>&1 || true
    return 0
  }

  while IFS= read -r n; do
    [[ -n "$n" ]] || continue
    [[ "$n" == codex-*.json ]] || continue
    if ! grep -Fxq "$n" "$keep_tmp" 2>/dev/null; then
      p="$accounts_dir/$n"
      if [[ -f "$p" ]]; then
        rm -f "$p" >/dev/null 2>&1 || true
        [[ ! -f "$p" ]] && deleted=$((deleted + 1))
      fi
    fi
  done < "$before_list"

  rm -f "$keep_tmp" >/dev/null 2>&1 || true
  echo "$deleted"
}

probe_worker() {
  local file="$1"
  local base="$2"
  local prefix="$3"
  local mode="$4"
  local cto="$5"
  local mto="$6"

  local type token aid email
  {
    IFS= read -r type || true
    IFS= read -r token || true
    IFS= read -r aid || true
    IFS= read -r email || true
  } < <(json_auth_fields4 "$file" 2>/dev/null || true)

  if [[ "$type" != "codex" || -z "$token" ]]; then
    echo "0|0|0" > "${prefix}.meta"
    : > "${prefix}.done"
    exit 0
  fi

  local status="000"
  local wham_body
  wham_body="${prefix}.wham"

  do_probe_once() {
    local proxy_arg=""
    if [[ "$1" == "direct" ]]; then
      proxy_arg="--noproxy *"
    fi
    if [[ -n "$aid" ]]; then
      curl -sS $proxy_arg --connect-timeout "$cto" --max-time "$mto" -o "$wham_body" -w "%{http_code}" \
        -H "Authorization: Bearer $token" \
        -H "Chatgpt-Account-Id: $aid" \
        -H "Accept: application/json, text/plain, */*" \
        -H "User-Agent: codex_cli_rs/0.76.0 (macOS/Linux)" \
        "https://chatgpt.com/backend-api/wham/usage" 2>/dev/null || true
    else
      curl -sS $proxy_arg --connect-timeout "$cto" --max-time "$mto" -o "$wham_body" -w "%{http_code}" \
        -H "Authorization: Bearer $token" \
        -H "Accept: application/json, text/plain, */*" \
        -H "User-Agent: codex_cli_rs/0.76.0 (macOS/Linux)" \
        "https://chatgpt.com/backend-api/wham/usage" 2>/dev/null || true
    fi
  }

  status="$(do_probe_once "$mode")"
  [[ -z "$status" || ! "$status" =~ ^[0-9]{3}$ ]] && status="000"

  if [[ "$mode" == "auto" && "$status" == "000" ]]; then
    status="$(do_probe_once "direct")"
    [[ -z "$status" || ! "$status" =~ ^[0-9]{3}$ ]] && status="000"
  fi

  status="$(json_normalize_wham_status "$status" "$wham_body")"
  rm -f "$wham_body" >/dev/null 2>&1 || true

  if [[ "$status" == "000" ]]; then
    echo "0|1|0" > "${prefix}.meta"
    echo "$base" > "${prefix}.net"
    : > "${prefix}.done"
    exit 0
  fi

  local now ident eh replay=false
  now="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  if [[ -n "$email" ]]; then
    ident="email:$(printf '%s' "$email" | tr '[:upper:]' '[:lower:]')"
  else
    ident="account_id:$aid"
  fi

  if command -v sha256sum >/dev/null 2>&1; then
    eh="$(printf '%s' "$ident" | sha256sum | awk '{print $1}')"
  elif command -v shasum >/dev/null 2>&1; then
    eh="$(printf '%s' "$ident" | shasum -a 256 | awk '{print $1}')"
  else
    eh="$(printf '%s' "$ident" | openssl dgst -sha256 | awk '{print $NF}')"
  fi

  if [[ -f "$REPLAY_QUEUE_FILE" ]] && grep -Fxq "$base" "$REPLAY_QUEUE_FILE" 2>/dev/null; then
    replay=true
  fi

  printf '{"file_name":"%s","email_hash":"%s","account_id":"%s","status_code":%s,"probed_at":"%s","replay_from_confidence":%s}\n' \
    "$base" "$eh" "$aid" "$status" "$now" "$replay" > "${prefix}.rep"

  local inv=0
  if [[ "$status" == "401" || "$status" == "429" ]]; then
    inv=1
  fi
  echo "1|0|$inv" > "${prefix}.meta"
  : > "${prefix}.done"
}

if [[ "$MODE_PROBE_WORKER" == "1" ]]; then
  probe_worker "$PROBE_FILE" "$PROBE_BASE" "$PROBE_PREFIX" "$WHAM_PROXY_MODE" "$WHAM_CONNECT_TIMEOUT" "$WHAM_MAX_TIME"
  exit 0
fi

count_replay_pending() {
  [[ -f "$REPLAY_QUEUE_FILE" ]] || {
    echo 0
    return 0
  }

  awk -v acc="$ACCOUNTS_DIR" 'NF>0 && !seen[$0]++ {
    cmd = "test -f \"" acc "/" $0 "\""
    if (system(cmd) == 0) print $0
  }' "$REPLAY_QUEUE_FILE" > "$REPLAY_QUEUE_FILE.tmp" 2>/dev/null || true

  if [[ -f "$REPLAY_QUEUE_FILE.tmp" ]]; then
    mv -f "$REPLAY_QUEUE_FILE.tmp" "$REPLAY_QUEUE_FILE"
  fi

  awk 'NF>0{c++} END{print c+0}' "$REPLAY_QUEUE_FILE" 2>/dev/null || echo 0
}

delete_invalid_from_reports() {
  local report_jsonl="$1"
  local backup_dir="$2"
  local response_scope_file="$3"
  local parser del_count skip_tmp del_tmp
  parser="$(detect_json_parser)"
  del_count=0
  skip_tmp="${backup_dir}/.skip.list"
  del_tmp="${backup_dir}/.del.list"

  mkdir -p "$backup_dir"
  [[ -f "$response_scope_file" ]] && awk 'NF>0{print}' "$response_scope_file" | awk '!seen[$0]++' > "$skip_tmp" || : > "$skip_tmp"

  if [[ "$parser" == "jq" ]]; then
    jq -r 'fromjson? | select((.status_code // 0)==401 or (.status_code // 0)==429) | .file_name // empty' "$report_jsonl" 2>/dev/null \
      | awk 'NF>0 && !seen[$0]++{print}' > "$del_tmp" || : > "$del_tmp"
  elif [[ "$parser" == "osascript" ]]; then
    osascript -l JavaScript <<OSA > "$del_tmp" 2>/dev/null || : > "$del_tmp"
ObjC.import('Foundation');
function readText(path){
  try { return $.NSString.stringWithContentsOfFileEncodingError(path, $.NSUTF8StringEncoding, null).js; }
  catch(e){ return ''; }
}
const lines = readText('$report_jsonl').split(/\r?\n/);
const seen = {};
for (const ln of lines) {
  if (!ln) continue;
  try {
    const o = JSON.parse(ln);
    const sc = Number(o.status_code || 0);
    const fn = String(o.file_name || '').trim();
    if ((sc === 401 || sc === 429) && fn && !seen[fn]) {
      seen[fn] = 1;
      console.log(fn);
    }
  } catch(e) {}
}
OSA
  else
    python3 - "$report_jsonl" "$ACCOUNTS_DIR" "$backup_dir" "$response_scope_file" <<'PY'
import json, os, sys, shutil
rep, acc, backup, scope = sys.argv[1:5]
os.makedirs(backup, exist_ok=True)
skip = set()
if os.path.exists(scope):
  with open(scope, 'r', encoding='utf-8') as f:
    for ln in f:
      n = ln.strip()
      if n:
        skip.add(n)
del_count = 0
if os.path.exists(rep):
  with open(rep, 'r', encoding='utf-8') as f:
    for ln in f:
      ln = ln.strip()
      if not ln:
        continue
      try:
        o = json.loads(ln)
      except Exception:
        continue
      sc = int(o.get('status_code') or 0)
      if sc not in (401, 429):
        continue
      fn = str(o.get('file_name') or '').strip()
      if not fn or fn in skip:
        continue
      src = os.path.join(acc, fn)
      if not os.path.exists(src):
        continue
      try:
        shutil.copy2(src, os.path.join(backup, fn))
      except Exception:
        pass
      try:
        os.remove(src)
      except Exception:
        pass
      if not os.path.exists(src):
        del_count += 1
print(del_count)
PY
    return 0
  fi

  while IFS= read -r fn; do
    [[ -n "$fn" ]] || continue
    if grep -Fxq "$fn" "$skip_tmp" 2>/dev/null; then
      continue
    fi
    src="$ACCOUNTS_DIR/$fn"
    [[ -f "$src" ]] || continue
    cp -f "$src" "$backup_dir/$fn" >/dev/null 2>&1 || true
    rm -f "$src" >/dev/null 2>&1 || true
    [[ ! -f "$src" ]] && del_count=$((del_count + 1))
  done < "$del_tmp"

  rm -f "$skip_tmp" "$del_tmp" >/dev/null 2>&1 || true
  echo "$del_count"
}

sync_all_flow() {
  local ts out_dir resp_json before_list
  local sync_cto sync_mto
  ts="$(date -u +%Y%m%d-%H%M%S)"

  if [[ "$RUN_OUTPUT_MODE" == "compact" ]]; then
    out_dir="$ROOT_DIR/out/latest-syncall"
    rm -rf "$out_dir" >/dev/null 2>&1 || true
  else
    out_dir="${TMPDIR:-/tmp}/InfiniteRefill-syncall-$ts"
  fi

  mkdir -p "$out_dir"
  resp_json="$out_dir/sync_all_response.json"
  before_list="$out_dir/sync_all_before_files.txt"
  : > "$before_list"
  for _f in "$ACCOUNTS_DIR"/*.json; do
    [[ -f "$_f" ]] || continue
    basename "$_f" >> "$before_list"
  done

  sync_cto="$TOPUP_CONNECT_TIMEOUT"
  sync_mto="$TOPUP_MAX_TIME"
  [[ "$sync_cto" =~ ^[0-9]+$ ]] || sync_cto=25
  [[ "$sync_mto" =~ ^[0-9]+$ ]] || sync_mto=300
  (( sync_cto < 25 )) && sync_cto=25
  (( sync_mto < 180 )) && sync_mto=180

  echo "[INFO] 全量同步：POST $SERVER_URL/v1/refill/sync-all"
  if ! curl -sS --connect-timeout "$sync_cto" --max-time "$sync_mto" --retry "$TOPUP_RETRY" --retry-all-errors --retry-delay "$TOPUP_RETRY_DELAY" --noproxy "*" \
      -X POST "$SERVER_URL/v1/refill/sync-all" \
      -H "X-User-Key: $USER_KEY" \
      -H "Content-Type: application/json" \
      --data-binary "{}" > "$resp_json"; then
    echo "[ERROR] sync-all 请求失败"
    return 2
  fi

  local parse_failed=0 server_count=0 written=0 failed=0
  while IFS='=' read -r k v; do
    case "$k" in
      ERROR) parse_failed=1; echo "[ERROR] sync-all failed: $v" ;;
      SERVER_COUNT) server_count="$v" ;;
      WRITTEN) written="$v" ;;
      WRITE_FAILED) failed="$v" ;;
    esac
  done < <(parse_topup_response "$resp_json" "$ACCOUNTS_DIR" "$REPLAY_QUEUE_FILE" 3 30 2>/dev/null || true)

  if [[ "$parse_failed" == "1" ]]; then
    return 2
  fi

  if (( server_count > written )); then
    local rec1 fail1 rec2 fail2
    rec1=0; fail1=0; rec2=0; fail2=0
    IFS='|' read -r rec1 fail1 < <(rescue_missing_from_response "$resp_json" "$ACCOUNTS_DIR" 6 60 2>/dev/null || echo "0|0")
    written=$((written + rec1))
    IFS='|' read -r rec2 fail2 < <(rescue_missing_from_response "$resp_json" "$ACCOUNTS_DIR" 6 90 2>/dev/null || echo "0|0")
    written=$((written + rec2))
    failed="$fail2"
    if (( rec1 > 0 )); then echo "[INFO] sync-all 二次补拉成功：$rec1"; fi
    if (( rec2 > 0 )); then echo "[INFO] sync-all 三次补拉成功：$rec2"; fi
  fi

  local deleted_old
  deleted_old="$(cleanup_old_codex_after_sync "$before_list" "$resp_json" "$ACCOUNTS_DIR" 2>/dev/null || echo 0)"
  [[ -z "$deleted_old" ]] && deleted_old=0

  echo "[INFO] 服务端返回账号条数：$server_count"
  echo "[INFO] 已写入账号：${written}（失败=${failed}）"
  echo "[INFO] sync-all 清理旧文件：$deleted_old"
  sync_managed_json
  echo "[OK] 全量同步完成"
  return 0
}

if [[ "$MODE_SYNC_ALL" == "1" ]]; then
  if ! sync_all_flow; then
    exit 2
  fi
fi

if [[ "$MODE_FROM_TASK" == "0" ]]; then
  echo "[INFO] 服务器地址=$SERVER_URL"
  echo "[INFO] accounts-dir=$ACCOUNTS_DIR"
  echo "[INFO] 目标账户数=$TARGET_POOL_SIZE 总持有上限=$TOTAL_HOLD_LIMIT 触发规则=存在失效账号即续杯"
  echo "[INFO] 并行探测=$PROBE_PARALLEL WHAM_PROXY_MODE=$WHAM_PROXY_MODE"
fi

if [[ -f "$ITER_SCOPE_FILE" ]]; then rm -f "$ITER_SCOPE_FILE" >/dev/null 2>&1 || true; fi
if [[ -f "$NEXT_SCOPE_FILE" ]]; then rm -f "$NEXT_SCOPE_FILE" >/dev/null 2>&1 || true; fi

REFILL_ITER=0
LAST_OUT_DIR="$ROOT_DIR/out/latest"

while (( REFILL_ITER < REFILL_ITER_MAX )); do
  REFILL_ITER=$((REFILL_ITER + 1))
  [[ "$MODE_FROM_TASK" == "0" ]] && echo "[INFO] 续杯闭环轮次：$REFILL_ITER/$REFILL_ITER_MAX"

  TS="$(date -u +%Y%m%d-%H%M%S)"
  if [[ "$RUN_OUTPUT_MODE" == "compact" ]]; then
    OUT_DIR="$ROOT_DIR/out/latest"
    rm -rf "$OUT_DIR" >/dev/null 2>&1 || true
  else
    OUT_DIR="$ROOT_DIR/out/单次续杯-$TS"
  fi
  mkdir -p "$OUT_DIR"
  LAST_OUT_DIR="$OUT_DIR"

  REPORT_JSONL="$OUT_DIR/reports.jsonl"
  RESP_JSON="$OUT_DIR/topup_response.json"
  BODY_JSON="$OUT_DIR/topup_body.json"
  BACKUP_DIR="$OUT_DIR/backup"
  NETFAIL_LOG="$OUT_DIR/probe_netfail.log"
  RESPONSE_SCOPE_FILE="$OUT_DIR/response_scope.txt"
  PROBE_DIR="$OUT_DIR/probe_jobs"
  mkdir -p "$BACKUP_DIR" "$PROBE_DIR"
  : > "$REPORT_JSONL"
  : > "$NETFAIL_LOG"

  total=0
  probed_ok=0
  net_fail=0
  invalid_401=0
  invalid_429=0

  SCOPE_MODE=0
  if (( REFILL_ITER > 1 )) && [[ -f "$ITER_SCOPE_FILE" ]] && [[ -s "$ITER_SCOPE_FILE" ]]; then
    SCOPE_MODE=1
  fi

  declare -a PROBE_FILES=()
  if (( SCOPE_MODE == 1 )); then
    [[ "$MODE_FROM_TASK" == "0" ]] && echo "[INFO] 增量探测模式（仅本轮新增/待确认；并行=${PROBE_PARALLEL}）"
    while IFS= read -r name; do
      [[ -n "$name" ]] || continue
      f="$ACCOUNTS_DIR/$name"
      [[ -f "$f" ]] || continue
      PROBE_FILES+=("$f")
    done < "$ITER_SCOPE_FILE"
  else
    [[ "$MODE_FROM_TASK" == "0" ]] && echo "[INFO] 全量探测模式（并行=${PROBE_PARALLEL}）"
    shopt -s nullglob
    for f in "$ACCOUNTS_DIR"/*.json; do
      [[ -f "$f" ]] || continue
      PROBE_FILES+=("$f")
    done
  fi

  launched=0
  for f in "${PROBE_FILES[@]:-}"; do
    total=$((total + 1))
    base="$(basename "$f")"
    prefix="$PROBE_DIR/$total"
    bash "$SCRIPT_DIR/单次续杯.sh" --probe-one-worker "$f" "$base" "$prefix" "$WHAM_PROXY_MODE" "$WHAM_CONNECT_TIMEOUT" "$WHAM_MAX_TIME" >/dev/null 2>&1 &
    launched=$((launched + 1))

    while true; do
      running="$(jobs -pr | wc -l | tr -d ' ')"
      [[ -z "$running" ]] && running=0
      if (( running < PROBE_PARALLEL )); then
        break
      fi
      sleep 0.2
    done
  done

  wait || true

  if (( launched > 0 )); then
    for rep in "$PROBE_DIR"/*.rep; do
      [[ -f "$rep" ]] || continue
      cat "$rep" >> "$REPORT_JSONL"
    done
    for net in "$PROBE_DIR"/*.net; do
      [[ -f "$net" ]] || continue
      cat "$net" >> "$NETFAIL_LOG"
    done
    for meta in "$PROBE_DIR"/*.meta; do
      [[ -f "$meta" ]] || continue
      IFS='|' read -r a b c < "$meta"
      [[ -z "$a" ]] && a=0
      [[ -z "$b" ]] && b=0
      [[ -z "$c" ]] && c=0
      probed_ok=$((probed_ok + a))
      net_fail=$((net_fail + b))
      if (( c > 0 )); then
        if grep -q '"status_code":401' "${meta%.meta}.rep" 2>/dev/null; then
          invalid_401=$((invalid_401 + 1))
        elif grep -q '"status_code":429' "${meta%.meta}.rep" 2>/dev/null; then
          invalid_429=$((invalid_429 + 1))
        fi
      fi
    done
  fi

  invalid=$((invalid_401 + invalid_429))
  available_est=$((total - invalid))
  hold_limit="$TOTAL_HOLD_LIMIT"
  request_target=$((hold_limit - available_est))
  (( request_target < 0 )) && request_target=0

  replay_pending="$(count_replay_pending 2>/dev/null || echo 0)"

  echo "[INFO] 统计：total=$total available_est=$available_est probed_ok=$probed_ok net_fail=$net_fail invalid_401=$invalid_401 invalid_429=$invalid_429 invalid=$invalid replay_pending=$replay_pending hold_limit=$hold_limit request_target=$request_target"

  if (( total == 0 && REFILL_ITER == 1 )); then
    echo "[INFO] 本地账号为0，先执行一次 sync-all"
    if ! sync_all_flow; then
      write_final_report "topup" "sync_all_failed" "$total" "$probed_ok" "$net_fail" "$invalid" "$OUT_DIR"
      exit 2
    fi
    continue
  fi

  if (( REFILL_ITER > 1 && SCOPE_MODE == 0 && replay_pending == 0 )); then
    echo "[OK] 增量范围为空且无待确认回放，闭环结束。"
    write_final_report "topup" "incremental_done" "$total" "$probed_ok" "$net_fail" "$invalid" "$OUT_DIR"
    [[ "$RUN_OUTPUT_MODE" != "compact" ]] && cleanup_old_out
    exit 0
  fi

  need_trigger=0
  (( invalid > 0 )) && need_trigger=1
  (( total < TARGET_POOL_SIZE )) && need_trigger=1
  (( available_est < hold_limit )) && need_trigger=1
  (( replay_pending > 0 )) && need_trigger=1

  if (( need_trigger == 0 )); then
    echo "[OK] 未达到续杯条件：无需 topup"
    write_final_report "topup" "not_triggered" "$total" "$probed_ok" "$net_fail" "$invalid" "$OUT_DIR"
    [[ "$RUN_OUTPUT_MODE" != "compact" ]] && cleanup_old_out
    exit 0
  fi

  if (( request_target == 0 && replay_pending == 0 )); then
    echo "[OK] 无需续杯：持有量已达上限且无回放待确认"
    write_final_report "topup" "at_limit" "$total" "$probed_ok" "$net_fail" "$invalid" "$OUT_DIR"
    [[ "$RUN_OUTPUT_MODE" != "compact" ]] && cleanup_old_out
    exit 0
  fi

  parser="$(detect_json_parser)"
  case "$parser" in
    jq)
      reports_tmp="$OUT_DIR/body_reports.json"
      accids_tmp="$OUT_DIR/body_account_ids.json"
      jq -s 'map(select((.status_code // 0) == 401 or (.status_code // 0) == 429 or (.replay_from_confidence == true)))' \
        "$REPORT_JSONL" > "$reports_tmp" 2>/dev/null || echo '[]' > "$reports_tmp"
      find "$ACCOUNTS_DIR" -maxdepth 1 -type f -name 'codex-*.json' 2>/dev/null \
        | sed -e 's#^.*/codex-##' -e 's#\.json$##' \
        | awk 'NF>0 && !seen[$0]++' | head -n 500 \
        | jq -R . | jq -s . > "$accids_tmp" 2>/dev/null || echo '[]' > "$accids_tmp"
      jq -n --argjson target_pool_size "$request_target" \
        --slurpfile reports "$reports_tmp" \
        --slurpfile account_ids "$accids_tmp" \
        '{target_pool_size:$target_pool_size,reports:($reports[0] // []),account_ids:($account_ids[0] // [])}' > "$BODY_JSON"
      ;;
    osascript)
      TOPUP_REPORT_JSONL="$REPORT_JSONL" \
      TOPUP_BODY_JSON="$BODY_JSON" \
      TOPUP_ACCOUNTS_DIR="$ACCOUNTS_DIR" \
      TOPUP_REQUEST_TARGET="$request_target" \
      osascript -l JavaScript <<'OSA' >/dev/null 2>&1 || true
ObjC.import('Foundation');
ObjC.import('stdlib');
function readLines(path){
  try {
    const s = $.NSString.stringWithContentsOfFileEncodingError(path, $.NSUTF8StringEncoding, null).js;
    return s.split(/\r?\n/).filter(Boolean);
  } catch(e){ return []; }
}
const rep = $.getenv('TOPUP_REPORT_JSONL');
const out = $.getenv('TOPUP_BODY_JSON');
const accDir = $.getenv('TOPUP_ACCOUNTS_DIR');
const target = Number($.getenv('TOPUP_REQUEST_TARGET') || '0') || 0;
const reports = [];
for (const ln of readLines(rep)) {
  try {
    const o = JSON.parse(ln);
    const sc = Number(o.status_code || 0);
    if (sc === 401 || sc === 429 || o.replay_from_confidence === true) reports.push(o);
  } catch(e) {}
}
const fm = $.NSFileManager.defaultManager;
const items = fm.contentsOfDirectoryAtPathError(accDir, null);
const accountIds = [];
const seen = {};
if (items) {
  const arr = ObjC.deepUnwrap(items);
  for (const n of arr) {
    if (n.startsWith('codex-') && n.endsWith('.json')) {
      const aid = n.substring(6, n.length - 5).trim();
      if (aid && !seen[aid]) { seen[aid] = 1; accountIds.push(aid); }
      if (accountIds.length >= 500) break;
    }
  }
}
const body = {target_pool_size: target, reports: reports, account_ids: accountIds};
const text = JSON.stringify(body);
$(text).writeToFileAtomicallyEncodingError($(out), true, $.NSUTF8StringEncoding, null);
OSA
      ;;
    *)
      python3 - "$REPORT_JSONL" "$BODY_JSON" "$request_target" "$ACCOUNTS_DIR" <<'PY'
import json, os, sys
rep, out, target, acc_dir = sys.argv[1:5]
target = int(target)
reports = []
if os.path.exists(rep):
  with open(rep, 'r', encoding='utf-8') as f:
    for ln in f:
      ln = ln.strip()
      if not ln:
        continue
      try:
        o = json.loads(ln)
      except Exception:
        continue
      sc = int(o.get('status_code') or 0)
      if sc in (401, 429) or o.get('replay_from_confidence') is True:
        reports.append(o)
acc_ids = []
if os.path.isdir(acc_dir):
  for n in os.listdir(acc_dir):
    if n.startswith('codex-') and n.endswith('.json'):
      aid = n[6:-5].strip()
      if aid:
        acc_ids.append(aid)
acc_ids = list(dict.fromkeys(acc_ids))[:500]
with open(out, 'w', encoding='utf-8') as f:
  json.dump({'target_pool_size': target, 'reports': reports, 'account_ids': acc_ids}, f, ensure_ascii=False, separators=(',', ':'))
PY
      ;;
  esac

  if [[ ! -s "$BODY_JSON" ]]; then
    echo "[ERROR] 未生成 topup 请求体文件：$BODY_JSON"
    write_final_report "topup" "topup_body_missing" "$total" "$probed_ok" "$net_fail" "$invalid" "$OUT_DIR"
    [[ "$RUN_OUTPUT_MODE" != "compact" ]] && cleanup_old_out
    exit 2
  fi

  echo "[INFO] 触发 topup：POST $SERVER_URL/v1/refill/topup"
  if ! curl -sS --connect-timeout "$TOPUP_CONNECT_TIMEOUT" --max-time "$TOPUP_MAX_TIME" --retry "$TOPUP_RETRY" --retry-all-errors --retry-delay "$TOPUP_RETRY_DELAY" --noproxy "*" \
      -X POST "$SERVER_URL/v1/refill/topup" \
      -H "X-User-Key: $USER_KEY" \
      -H "Content-Type: application/json" \
      --data-binary "@$BODY_JSON" > "$RESP_JSON"; then
    echo "[ERROR] topup 请求失败"
    write_final_report "topup" "topup_request_failed" "$total" "$probed_ok" "$net_fail" "$invalid" "$OUT_DIR"
    [[ "$RUN_OUTPUT_MODE" != "compact" ]] && cleanup_old_out
    exit 2
  fi

  SERVER_ACCOUNTS_COUNT=0
  WRITTEN_COUNT=0
  WRITE_FAILED_COUNT=0
  REPORT_AUTO_DISABLED=""
  REPORT_ABUSE_AUTO_BANNED=""
  SERVER_HOLD_LIMIT=""

  parse_failed=0
  while IFS='=' read -r k v; do
    case "$k" in
      ERROR) parse_failed=1; echo "[ERROR] topup failed: $v" ;;
      SERVER_COUNT) SERVER_ACCOUNTS_COUNT="$v" ;;
      WRITTEN) WRITTEN_COUNT="$v" ;;
      WRITE_FAILED) WRITE_FAILED_COUNT="$v" ;;
      TOTAL_HOLD_LIMIT) SERVER_HOLD_LIMIT="$v" ;;
      AUTO_DISABLED) REPORT_AUTO_DISABLED="$v" ;;
      ABUSE_AUTO_BANNED) REPORT_ABUSE_AUTO_BANNED="$v" ;;
      CONFIDENCE_REPLAY_PERCENT) echo "[INFO] 待置信回放占比：$v%" ;;
      ISSUED_REPLAY_COUNT) echo "[INFO] 本次回放下发数量：$v" ;;
      DL_FAIL) echo "[WARN] 下载失败：$v" ;;
    esac
  done < <(parse_topup_response "$RESP_JSON" "$ACCOUNTS_DIR" "$REPLAY_QUEUE_FILE" 3 30 2>/dev/null || true)

  if (( parse_failed == 1 )); then
    write_final_report "topup" "topup_parse_failed" "$total" "$probed_ok" "$net_fail" "$invalid" "$OUT_DIR"
    [[ "$RUN_OUTPUT_MODE" != "compact" ]] && cleanup_old_out
    exit 2
  fi

  if (( SERVER_ACCOUNTS_COUNT > WRITTEN_COUNT )); then
    rec2=0; fail2=0; rec3=0; fail3=0
    IFS='|' read -r rec2 fail2 < <(rescue_missing_from_response "$RESP_JSON" "$ACCOUNTS_DIR" 6 60 2>/dev/null || echo "0|0")
    WRITTEN_COUNT=$((WRITTEN_COUNT + rec2))
    IFS='|' read -r rec3 fail3 < <(rescue_missing_from_response "$RESP_JSON" "$ACCOUNTS_DIR" 8 90 2>/dev/null || echo "0|0")
    WRITTEN_COUNT=$((WRITTEN_COUNT + rec3))
    WRITE_FAILED_COUNT="$fail3"
    if (( rec2 > 0 )); then echo "[INFO] topup 二次补拉成功：$rec2"; fi
    if (( rec3 > 0 )); then echo "[INFO] topup 三次补拉成功：$rec3"; fi
  fi

  echo "[INFO] 服务端返回账号条数：$SERVER_ACCOUNTS_COUNT"
  echo "[INFO] 写入新账号：$WRITTEN_COUNT"
  if (( WRITE_FAILED_COUNT > 0 )); then
    echo "[WARN] 新账号写入失败：$WRITE_FAILED_COUNT"
  fi

  if [[ -n "$SERVER_HOLD_LIMIT" && "$SERVER_HOLD_LIMIT" =~ ^[0-9]+$ ]]; then
    TOTAL_HOLD_LIMIT="$SERVER_HOLD_LIMIT"
    upsert_env_key "TOTAL_HOLD_LIMIT" "$SERVER_HOLD_LIMIT"
    echo "[INFO] 服务端下发总持有上限：$SERVER_HOLD_LIMIT"
  fi

  build_response_scope "$RESP_JSON" "$RESPONSE_SCOPE_FILE"
  DEL_COUNT="$(delete_invalid_from_reports "$REPORT_JSONL" "$BACKUP_DIR" "$RESPONSE_SCOPE_FILE" 2>/dev/null || echo 0)"
  [[ -z "$DEL_COUNT" ]] && DEL_COUNT=0
  echo "[INFO] 已删除失效账号文件：$DEL_COUNT"

  consume_replay_queue_by_reports "$REPORT_JSONL"

  sync_needed=$((WRITTEN_COUNT + DEL_COUNT))
  if (( sync_needed > 0 )); then
    sync_managed_json
  fi

  TOPUP_STATUS="triggered"
  report_auto_disabled_lc="$(printf '%s' "${REPORT_AUTO_DISABLED:-}" | tr '[:upper:]' '[:lower:]')"
  report_abuse_auto_banned_lc="$(printf '%s' "${REPORT_ABUSE_AUTO_BANNED:-}" | tr '[:upper:]' '[:lower:]')"
  case "$report_auto_disabled_lc" in
    true|1|yes) TOPUP_STATUS="server_auto_disabled" ;;
  esac
  case "$report_abuse_auto_banned_lc" in
    true|1|yes) TOPUP_STATUS="server_abuse_auto_banned" ;;
  esac

  if [[ "$TOPUP_STATUS" == "server_auto_disabled" ]]; then
    write_final_report "topup" "$TOPUP_STATUS" "$total" "$probed_ok" "$net_fail" "$invalid" "$OUT_DIR"
    [[ "$RUN_OUTPUT_MODE" != "compact" ]] && cleanup_old_out
    exit 4
  fi
  if [[ "$TOPUP_STATUS" == "server_abuse_auto_banned" ]]; then
    write_final_report "topup" "$TOPUP_STATUS" "$total" "$probed_ok" "$net_fail" "$invalid" "$OUT_DIR"
    [[ "$RUN_OUTPUT_MODE" != "compact" ]] && cleanup_old_out
    exit 5
  fi

  if (( WRITTEN_COUNT > 0 )) && [[ -f "$RESPONSE_SCOPE_FILE" ]]; then
    cp -f "$RESPONSE_SCOPE_FILE" "$NEXT_SCOPE_FILE" >/dev/null 2>&1 || true
  else
    rm -f "$NEXT_SCOPE_FILE" >/dev/null 2>&1 || true
  fi

  if [[ -f "$NEXT_SCOPE_FILE" ]]; then
    mv -f "$NEXT_SCOPE_FILE" "$ITER_SCOPE_FILE" >/dev/null 2>&1 || true
    [[ "$MODE_FROM_TASK" == "0" ]] && echo "[INFO] 本轮已完成，继续下一轮可用性校验..."
  else
    rm -f "$ITER_SCOPE_FILE" >/dev/null 2>&1 || true
  fi

done

if [[ "$MODE_FROM_TASK" == "0" ]]; then
  echo "[WARN] 已达到最大闭环轮次：$REFILL_ITER_MAX"
fi

write_final_report "topup" "max_iter_reached" "0" "0" "0" "0" "$LAST_OUT_DIR"
[[ "$RUN_OUTPUT_MODE" != "compact" ]] && cleanup_old_out
exit 0
