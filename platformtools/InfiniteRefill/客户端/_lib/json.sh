#!/usr/bin/env bash
# shellcheck shell=bash
set -euo pipefail

# 通用 JSON 处理工具（目标：macOS 用户不装 jq 也能跑）
# 优先级：python3 > osascript(JXA) > jq

have_cmd() {
  command -v "$1" >/dev/null 2>&1
}

need_cmd() {
  have_cmd "$1" || { echo "[ERROR] 缺少依赖：$1" >&2; return 3; }
}

need_json_parser() {
  if have_cmd python3 || have_cmd osascript || have_cmd jq; then
    return 0
  fi
  echo "[ERROR] 缺少 JSON 解析器：需要 python3 或 osascript(仅 macOS) 或 jq" >&2
  return 3
}

# 输出 4 行：type / access_token / account_id / email（不存在则空行）
json_auth_fields4() {
  local f="$1"

  if have_cmd python3; then
    python3 - "$f" <<'PY'
import json,sys
p=sys.argv[1]
try:
  with open(p,'r',encoding='utf-8-sig') as fp:
    d=json.load(fp)
except Exception:
  sys.exit(1)

def s(v):
  if v is None:
    return ''
  if isinstance(v,(dict,list)):
    return json.dumps(v, ensure_ascii=False, separators=(',',':'))
  return str(v)

print(s(d.get('type','')))
print(s(d.get('access_token','')))
print(s(d.get('account_id','')))
print(s(d.get('email','')))
PY
    return $?
  fi

  if have_cmd osascript; then
    AUTH_FILE="$f" osascript -l JavaScript <<'JXA'
ObjC.import('Foundation');
function readUtf8(path){
  const s = $.NSString.stringWithContentsOfFileEncodingError($(path), $.NSUTF8StringEncoding, null);
  return ObjC.unwrap(s);
}
function s(v){
  if (v === null || v === undefined) return '';
  if (typeof v === 'object') return JSON.stringify(v);
  return String(v);
}
const path = ObjC.unwrap($.getenv('AUTH_FILE'));
let d = {};
try { d = JSON.parse(readUtf8(path)); } catch(e) { $.exit(1); }
console.log(s(d.type));
console.log(s(d.access_token));
console.log(s(d.account_id));
console.log(s(d.email));
JXA
    return $?
  fi

  if have_cmd jq; then
    jq -r '[.type // "", .access_token // "", .account_id // "", .email // ""] | .[]' "$f" 2>/dev/null
    return $?
  fi

  echo "[ERROR] 缺少 JSON 解析器：需要 python3 或 osascript 或 jq" >&2
  return 3
}

json_check_no_sensitive_keys() {
  # 已禁用“敏感字段”检测：允许包含 access_token/refresh_token/id_token。
  # 保留该函数仅为兼容旧脚本调用点。
  return 0
}

# 校验“热心群众”提交 JSON 的合规性。成功：return 0；失败：打印原因并 return 非0。
# mode: register | report
json_check_payload_file() {
  local mode="$1"
  local f="$2"

  # 已禁用：敏感字段检查（允许 token 字段）

  need_json_parser >/dev/null 2>&1 || { echo "缺少 JSON 解析器"; return 3; }

  if have_cmd python3; then
    python3 - "$mode" "$f" <<'PY'
import json, re, sys
mode=sys.argv[1]
p=sys.argv[2]

try:
  with open(p,'r',encoding='utf-8') as fp:
    d=json.load(fp)
except Exception:
  print('不是合法JSON')
  sys.exit(9)

HEX64=re.compile(r'^[a-f0-9]{64}$', re.I)

def is_ok_item_register(it):
  if not isinstance(it, dict):
    return False
  eh=it.get('email_hash')
  sa=it.get('seen_at')
  return isinstance(eh,str) and HEX64.match(eh or '') and isinstance(sa,str) and len(sa)>0

def is_ok_item_report(it):
  if not isinstance(it, dict):
    return False
  eh=it.get('email_hash')
  pa=it.get('probed_at')
  sc=it.get('status_code')
  if not (isinstance(eh,str) and HEX64.match(eh or '') and isinstance(pa,str) and len(pa)>0):
    return False
  return isinstance(sc,(int,float,str))

if mode=='register':
  acc=d.get('accounts')
  if not isinstance(acc, list):
    print('缺少 accounts[]')
    sys.exit(9)
  if any((not is_ok_item_register(it)) for it in acc):
    print('accounts[] 内字段不合规')
    sys.exit(9)
  sys.exit(0)

if mode=='report':
  rep=d.get('reports')
  if not isinstance(rep, list):
    print('缺少 reports[]')
    sys.exit(9)
  if any((not is_ok_item_report(it)) for it in rep):
    print('reports[] 内字段不合规')
    sys.exit(9)
  sys.exit(0)

print('接口参数只能是 register 或 report')
sys.exit(2)
PY
    return $?
  fi

  if have_cmd osascript; then
    MODE="$mode" JSON_FILE="$f" osascript -l JavaScript <<'JXA'
ObjC.import('Foundation');
function readUtf8(path){
  const s = $.NSString.stringWithContentsOfFileEncodingError($(path), $.NSUTF8StringEncoding, null);
  return ObjC.unwrap(s);
}
const mode = ObjC.unwrap($.getenv('MODE'));
const path = ObjC.unwrap($.getenv('JSON_FILE'));
let d;
try { d = JSON.parse(readUtf8(path)); } catch(e) { console.log('不是合法JSON'); $.exit(9); }

function isHex64(s){
  return (typeof s === 'string') && /^[a-f0-9]{64}$/i.test(s);
}
function nonEmptyStr(s){
  return (typeof s === 'string') && s.length>0;
}

if (mode === 'register') {
  const acc = d.accounts;
  if (!Array.isArray(acc)) { console.log('缺少 accounts[]'); $.exit(9); }
  for (let i=0;i<acc.length;i++) {
    const it = acc[i];
    if (!it || typeof it !== 'object' || Array.isArray(it)) { console.log('accounts[] 内字段不合规'); $.exit(9); }
    if (!isHex64(it.email_hash) || !nonEmptyStr(it.seen_at)) { console.log('accounts[] 内字段不合规'); $.exit(9); }
  }
  $.exit(0);
}

if (mode === 'report') {
  const rep = d.reports;
  if (!Array.isArray(rep)) { console.log('缺少 reports[]'); $.exit(9); }
  for (let i=0;i<rep.length;i++) {
    const it = rep[i];
    if (!it || typeof it !== 'object' || Array.isArray(it)) { console.log('reports[] 内字段不合规'); $.exit(9); }
    if (!isHex64(it.email_hash) || !nonEmptyStr(it.probed_at)) { console.log('reports[] 内字段不合规'); $.exit(9); }
    const sc = it.status_code;
    if (!(typeof sc === 'number' || typeof sc === 'string')) { console.log('reports[] 内字段不合规'); $.exit(9); }
  }
  $.exit(0);
}

console.log('接口参数只能是 register 或 report');
$.exit(2);
JXA
    return $?
  fi

  # jq 最后兜底
  if have_cmd jq; then
    if ! jq -e . "$f" >/dev/null 2>&1; then
      echo "不是合法JSON"
      return 9
    fi

    if [[ "$mode" == "register" ]]; then
      jq -e '.accounts and (.accounts|type=="array")' "$f" >/dev/null 2>&1 || { echo "缺少 accounts[]"; return 9; }
      jq -e '.accounts[] | (.email_hash|type=="string" and test("^[a-f0-9]{64}$";"i")) and (.seen_at|type=="string" and length>0)' "$f" >/dev/null 2>&1 || { echo "accounts[] 内字段不合规"; return 9; }
      return 0
    fi

    if [[ "$mode" == "report" ]]; then
      jq -e '.reports and (.reports|type=="array")' "$f" >/dev/null 2>&1 || { echo "缺少 reports[]"; return 9; }
      jq -e '.reports[] | (.email_hash|type=="string" and test("^[a-f0-9]{64}$";"i")) and (.probed_at|type=="string" and length>0) and (.status_code|type=="number" or type=="string")' "$f" >/dev/null 2>&1 || { echo "reports[] 内字段不合规"; return 9; }
      return 0
    fi

    echo "接口参数只能是 register 或 report"
    return 2
  fi

  echo "缺少 JSON 解析器"
  return 3
}

# 解析 topup 响应并把 accounts[] 写入 out_dir；成功输出写入数量
# 兼容两种服务端返回：
# - 旧：accounts[].auth_json
# - 新：accounts[].download_url（预签名下载链接）
json_topup_write_accounts_from_response() {
  local resp_file="$1"
  local out_dir="$2"

  need_json_parser >/dev/null 2>&1 || return 3
  need_cmd curl >/dev/null 2>&1 || return 3

  if have_cmd python3; then
    python3 - "$resp_file" "$out_dir" <<'PY'
import json, os, random, sys, time, urllib.request
resp=sys.argv[1]
out_dir=sys.argv[2]

try:
  with open(resp,'r',encoding='utf-8') as fp:
    data=json.load(fp)
except Exception:
  print('invalid_json')
  sys.exit(2)

if not data.get('ok'):
  print(data.get('error','unknown'))
  sys.exit(2)

accounts=data.get('accounts') or []
written=0
for item in accounts:
  if not isinstance(item, dict):
    continue

  fn=item.get('file_name') or f"无限续杯-{int(time.time())}-{random.randint(1000,9999)}.json"
  path=os.path.join(out_dir, fn)

  auth=item.get('auth_json')
  if auth is not None:
    with open(path,'w',encoding='utf-8') as out:
      json.dump(auth, out, ensure_ascii=False, indent=2)
      out.write('\n')
    written += 1
    continue

  dl=(item.get('download_url') or '').strip()
  if dl:
    try:
      req=urllib.request.Request(dl, method='GET')
      with urllib.request.urlopen(req, timeout=30) as r:
        raw=r.read()
      text=raw.decode('utf-8-sig', errors='strict')
      parsed=json.loads(text)
      canon=json.dumps(parsed, ensure_ascii=False, separators=(',',':'), sort_keys=True)
      with open(path,'w',encoding='utf-8') as out:
        out.write(canon)
        out.write('\n')
      written += 1
    except Exception:
      continue

print(str(written))
PY
    return $?
  fi

  if have_cmd osascript; then
    RESP_FILE="$resp_file" OUT_DIR="$out_dir" osascript -l JavaScript <<'JXA'
ObjC.import('Foundation');
function readUtf8(path){
  const s = $.NSString.stringWithContentsOfFileEncodingError($(path), $.NSUTF8StringEncoding, null);
  return ObjC.unwrap(s);
}
function writeUtf8(path, text){
  const ns = $(text);
  ns.writeToFileAtomicallyEncodingError($(path), true, $.NSUTF8StringEncoding, null);
}
function fetchText(urlStr){
  const nsUrl = $.NSURL.URLWithString($(urlStr));
  if (!nsUrl) return null;
  const data = $.NSData.dataWithContentsOfURL(nsUrl);
  if (!data) return null;
  const text = $.NSString.alloc.initWithDataEncoding(data, $.NSUTF8StringEncoding);
  return text ? ObjC.unwrap(text) : null;
}
const resp = ObjC.unwrap($.getenv('RESP_FILE'));
const outDir = ObjC.unwrap($.getenv('OUT_DIR'));
let data;
try { data = JSON.parse(readUtf8(resp)); } catch(e) { console.log('invalid_json'); $.exit(2); }
if (!data.ok) { console.log(data.error || 'unknown'); $.exit(2); }
const accounts = data.accounts || [];
let written = 0;
for (let i=0;i<accounts.length;i++) {
  const item = accounts[i] || {};
  const fn = item.file_name || ('无限续杯-' + Math.floor(Date.now()/1000) + '-' + Math.floor(Math.random()*10000) + '.json');
  const path = outDir + '/' + fn;

  const auth = item.auth_json;
  if (auth !== undefined && auth !== null) {
    writeUtf8(path, JSON.stringify(auth, null, 2) + '\n');
    written++;
    continue;
  }

  const dl = (item.download_url || '').toString().trim();
  if (dl) {
    const t = fetchText(dl);
    if (t !== null) {
      try {
        const t2 = (t.charCodeAt(0) === 0xFEFF) ? t.slice(1) : t;
        const canon = JSON.stringify(JSON.parse(t2), null, 2) + '\n';
        writeUtf8(path, canon);
        written++;
      } catch(e) {
        // 非法 JSON 则跳过
      }
    }
  }
}
console.log(String(written));
JXA
    return $?
  fi

  if have_cmd jq; then
    ok="$(jq -r '.ok // false' "$resp_file" 2>/dev/null || echo false)"
    if [[ "$ok" != "true" ]]; then
      jq -r '.error // "unknown"' "$resp_file" 2>/dev/null || echo unknown
      return 2
    fi

    local written=0
    while IFS= read -r item; do
      fn="$(printf "%s" "$item" | jq -r '.file_name // empty')"
      [[ -z "$fn" ]] && fn="无限续杯-$(date -u +%s)-$RANDOM.json"

      has_auth="$(printf "%s" "$item" | jq -r 'has("auth_json") and (.auth_json != null)')"
      if [[ "$has_auth" == "true" ]]; then
        printf "%s" "$item" | jq '.auth_json' > "$out_dir/$fn"
        written=$((written+1))
        continue
      fi

      dl="$(printf "%s" "$item" | jq -r '.download_url // empty')"
      if [[ -n "$dl" ]]; then
        if curl -fsSL "$dl" | jq -cS . > "$out_dir/$fn" 2>/dev/null; then
          printf '\n' >> "$out_dir/$fn"
          written=$((written+1))
        fi
      fi
    done < <(jq -c '.accounts[]?' "$resp_file")

    echo "$written"
    return 0
  fi

  echo "缺少 JSON 解析器"
  return 3
}
