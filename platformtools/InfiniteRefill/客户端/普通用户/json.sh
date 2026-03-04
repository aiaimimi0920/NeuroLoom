#!/usr/bin/env bash
# JSON 解析工具库
# 支持 python3（推荐）/ osascript(JXA, macOS) / jq（兜底）

# 检查命令是否存在
need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "[ERROR] 缺少必需命令：$1" >&2
    exit 1
  fi
}

# 检测可用的 JSON 解析器
detect_json_parser() {
  if command -v python3 >/dev/null 2>&1; then
    echo "python3"
  elif command -v osascript >/dev/null 2>&1; then
    # 检查是否为 macOS
    if [[ "$(uname)" == "Darwin" ]]; then
      echo "osascript"
    elif command -v jq >/dev/null 2>&1; then
      echo "jq"
    else
      echo "none"
    fi
  elif command -v jq >/dev/null 2>&1; then
    echo "jq"
  else
    echo "none"
  fi
}

# 检查 JSON 解析器是否可用
need_json_parser() {
  local parser
  parser="$(detect_json_parser)"
  if [[ "$parser" == "none" ]]; then
    echo "[ERROR] 未找到 JSON 解析器，请安装：python3（推荐）或 jq" >&2
    exit 1
  fi
}

# 从 JSON 文件中提取认证字段（返回 4 行：type, token, account_id, email）
json_auth_fields4() {
  local file="$1"
  local parser
  parser="$(detect_json_parser)"

  case "$parser" in
    python3)
      python3 -c "
import json, sys
try:
    with open('$file', 'r', encoding='utf-8') as f:
        data = json.load(f)

    # 提取 type
    auth_type = data.get('type', '')
    print(auth_type)

    # 提取 token
    token = data.get('access_token', '')
    print(token)

    # 提取 account_id
    account_id = data.get('account_id', '')
    print(account_id)

    # 提取 email
    email = data.get('email', '')
    print(email)
except Exception as e:
    sys.exit(1)
" 2>/dev/null
      ;;

    osascript)
      osascript -l JavaScript -e "
const fs = require('fs');
try {
  const data = JSON.parse(fs.readFileSync('$file', 'utf8'));
  console.log(data.type || '');
  console.log(data.access_token || '');
  console.log(data.account_id || '');
  console.log(data.email || '');
} catch(e) {
  $.exit(1);
}
" 2>/dev/null
      ;;

    jq)
      jq -r '.type // "", .access_token // "", .account_id // "", .email // ""' "$file" 2>/dev/null
      ;;

    *)
      return 1
      ;;
  esac
}

# 规范化 wham 状态码
json_normalize_wham_status() {
  local status="$1"
  local body_file="$2"
  local parser
  parser="$(detect_json_parser)"

  # 如果状态码不是 200，直接返回
  if [[ "$status" != "200" ]]; then
    echo "$status"
    return 0
  fi

  # 检查响应体中是否有错误信息，可能需要映射为 429
  if [[ -f "$body_file" ]]; then
    local has_quota_error=0

    case "$parser" in
      python3)
        has_quota_error=$(python3 -c "
import json, sys
try:
    with open('$body_file', 'r', encoding='utf-8') as f:
        data = json.load(f)

    # 检查是否有配额耗尽的错误
    if isinstance(data, dict):
        error = data.get('error', {})
        if isinstance(error, dict):
            code = error.get('code', '')
            if 'quota' in str(code).lower() or 'limit' in str(code).lower():
                print('1')
                sys.exit(0)

        # 检查其他可能的配额字段
        if data.get('is_quota_exhausted') or data.get('quota_exhausted'):
            print('1')
            sys.exit(0)

    print('0')
except:
    print('0')
" 2>/dev/null)
        ;;

      osascript)
        has_quota_error=$(osascript -l JavaScript -e "
const fs = require('fs');
try {
  const data = JSON.parse(fs.readFileSync('$body_file', 'utf8'));
  if (data.error && (String(data.error.code).toLowerCase().includes('quota') ||
                     String(data.error.code).toLowerCase().includes('limit'))) {
    console.log('1');
  } else if (data.is_quota_exhausted || data.quota_exhausted) {
    console.log('1');
  } else {
    console.log('0');
  }
} catch(e) {
  console.log('0');
}
" 2>/dev/null)
        ;;

      jq)
        has_quota_error=$(jq -r '
          if (.error.code | tostring | ascii_downcase | contains("quota")) or
             (.error.code | tostring | ascii_downcase | contains("limit")) or
             .is_quota_exhausted or .quota_exhausted
          then "1" else "0" end
        ' "$body_file" 2>/dev/null || echo "0")
        ;;

      *)
        has_quota_error=0
        ;;
    esac

    if [[ "$has_quota_error" == "1" ]]; then
      echo "429"
      return 0
    fi
  fi

  echo "$status"
}

# 从 topup 响应中写入账号文件
json_topup_write_accounts_from_response() {
  local resp_file="$1"
  local accounts_dir="$2"
  local parser
  parser="$(detect_json_parser)"

  # 检查响应文件是否存在
  if [[ ! -f "$resp_file" ]]; then
    echo "[ERROR] 响应文件不存在：$resp_file" >&2
    return 1
  fi

  case "$parser" in
    python3)
      python3 -c "
import json, sys, os
from urllib.request import urlopen, Request

try:
    with open('$resp_file', 'r', encoding='utf-8') as f:
        resp = json.load(f)

    # 检查响应是否成功
    if not resp.get('ok'):
        sys.exit(1)

    accounts = resp.get('accounts', [])
    count = 0

    for acc in accounts:
        file_name = acc.get('file_name', '')
        download_url = acc.get('download_url', '')

        if not file_name or not download_url:
            continue

        # 下载账号文件
        try:
            req = Request(download_url, headers={'User-Agent': 'curl/7.0'})
            with urlopen(req, timeout=30) as response:
                content = response.read()

            # 写入文件
            target_path = os.path.join('$accounts_dir', file_name)
            with open(target_path, 'wb') as f:
                f.write(content)

            count += 1
        except Exception as e:
            print(f'[WARN] 下载失败：{file_name} - {e}', file=sys.stderr)
            continue

    print(count)
except Exception as e:
    print(f'[ERROR] 解析响应失败：{e}', file=sys.stderr)
    sys.exit(1)
" 2>&1
      ;;

    osascript)
      osascript -l JavaScript -e "
const fs = require('fs');
const https = require('https');
const http = require('http');
const url = require('url');

function download(downloadUrl, targetPath) {
  return new Promise((resolve, reject) => {
    const parsedUrl = url.parse(downloadUrl);
    const client = parsedUrl.protocol === 'https:' ? https : http;

    client.get(downloadUrl, {headers: {'User-Agent': 'curl/7.0'}}, (res) => {
      if (res.statusCode !== 200) {
        reject(new Error('HTTP ' + res.statusCode));
        return;
      }

      const chunks = [];
      res.on('data', (chunk) => chunks.push(chunk));
      res.on('end', () => {
        try {
          fs.writeFileSync(targetPath, Buffer.concat(chunks));
          resolve();
        } catch(e) {
          reject(e);
        }
      });
    }).on('error', reject);
  });
}

try {
  const resp = JSON.parse(fs.readFileSync('$resp_file', 'utf8'));

  if (!resp.ok) {
    $.exit(1);
  }

  const accounts = resp.accounts || [];
  let count = 0;

  for (const acc of accounts) {
    const fileName = acc.file_name || '';
    const downloadUrl = acc.download_url || '';

    if (!fileName || !downloadUrl) continue;

    try {
      const targetPath = '$accounts_dir/' + fileName;
      download(downloadUrl, targetPath);
      count++;
    } catch(e) {
      console.error('[WARN] 下载失败：' + fileName + ' - ' + e);
    }
  }

  console.log(count);
} catch(e) {
  console.error('[ERROR] 解析响应失败：' + e);
  $.exit(1);
}
" 2>&1
      ;;

    jq)
      # jq 方案：先解析，再用 curl 下载
      local count=0
      local ok
      ok=$(jq -r '.ok // false' "$resp_file" 2>/dev/null)

      if [[ "$ok" != "true" ]]; then
        return 1
      fi

      local accounts_json
      accounts_json=$(jq -c '.accounts[]' "$resp_file" 2>/dev/null)

      while IFS= read -r acc; do
        [[ -z "$acc" ]] && continue

        local file_name download_url
        file_name=$(echo "$acc" | jq -r '.file_name // ""' 2>/dev/null)
        download_url=$(echo "$acc" | jq -r '.download_url // ""' 2>/dev/null)

        [[ -z "$file_name" || -z "$download_url" ]] && continue

        # 使用 curl 下载
        if curl -sS -L -A "curl/7.0" -o "$accounts_dir/$file_name" "$download_url" 2>/dev/null; then
          count=$((count + 1))
        else
          echo "[WARN] 下载失败：$file_name" >&2
        fi
      done <<< "$accounts_json"

      echo "$count"
      ;;

    *)
      return 1
      ;;
  esac
}