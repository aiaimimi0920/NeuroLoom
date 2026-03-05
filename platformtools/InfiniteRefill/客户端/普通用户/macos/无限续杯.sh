#!/usr/bin/env bash
set -euo pipefail

# 设计目标：Unix 版 1:1 对齐 Windows 入口能力（macOS + Ubuntu）
# - 交互配置
# - 单次续杯
# - 定时任务启停（cron）
# - 自动清理 + 续杯串行
# - 同步账号
# - 失败自动停用（由 _定时任务_入口.sh 处理返回码 4/5）

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
if [ "$(basename "$SCRIPT_DIR")" = "unix" ]; then
  ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
else
  ROOT_DIR="$SCRIPT_DIR"
fi
CFG="$ROOT_DIR/无限续杯配置.env"
TASK_ENTRY="$SCRIPT_DIR/_定时任务_入口.sh"

REFILL_TASK_PREFIX="无限续杯_定时任务"

server_url_input="${1:-}"
user_key_input="${2:-}"

detect_os() {
  case "$(uname -s 2>/dev/null || echo unknown)" in
    Darwin) echo "macos" ;;
    Linux) echo "linux" ;;
    *) echo "other" ;;
  esac
}

print_install_hint() {
  local os="$1"
  echo "[HINT] 依赖缺失时可参考："
  if [[ "$os" == "linux" ]]; then
    echo "       Ubuntu/Debian: sudo apt install -y curl jq cron"
    echo "       CentOS/RHEL  : sudo yum install -y curl jq cronie"
    echo "       （可选兜底） : sudo apt/yum install -y python3"
  elif [[ "$os" == "macos" ]]; then
    echo "       macOS 通常自带 curl / crontab / osascript"
    echo "       若需要 jq：brew install jq"
    echo "       若需要 python3（兜底）：brew install python"
  else
    echo "       请安装 curl / (jq 或 osascript 或 python3) / crontab 后重试"
  fi
}

check_cmd() {
  command -v "$1" >/dev/null 2>&1
}

has_json_runtime() {
  local os
  os="$(detect_os)"
  if [[ "$os" == "macos" ]] && check_cmd osascript; then
    return 0
  fi
  if check_cmd jq; then
    return 0
  fi
  if check_cmd python3; then
    return 0
  fi
  return 1
}

ensure_runtime_base() {
  local os miss=0
  os="$(detect_os)"
  if ! check_cmd curl; then
    echo "[ERROR] 缺少依赖：curl"
    miss=1
  fi
  if ! has_json_runtime; then
    echo "[ERROR] 缺少 JSON 运行时：需要 jq（Linux 推荐）或 osascript（mac 推荐）或 python3（兜底）"
    miss=1
  fi
  if (( miss == 1 )); then
    print_install_hint "$os"
    return 1
  fi
  return 0
}

ensure_runtime_cron() {
  if ! check_cmd crontab; then
    echo "[ERROR] 缺少依赖：crontab（定时任务功能不可用）"
    print_install_hint "$(detect_os)"
    return 1
  fi
  return 0
}

self_check() {
  local os
  os="$(detect_os)"
  echo "[INFO] Unix 客户端环境自检"
  echo "[INFO] OS=$os"
  echo "[INFO] bash    : $(command -v bash || echo MISSING)"
  echo "[INFO] curl    : $(command -v curl || echo MISSING)"
  echo "[INFO] osascript: $(command -v osascript || echo MISSING)"
  echo "[INFO] jq      : $(command -v jq || echo MISSING)"
  echo "[INFO] python3 : $(command -v python3 || echo MISSING)"
  echo "[INFO] crontab : $(command -v crontab || echo MISSING)"

  if ensure_runtime_base; then
    echo "[OK] 基础依赖检查通过（curl + JSON运行时）"
  else
    echo "[FAIL] 基础依赖检查失败"
    return 1
  fi

  if ensure_runtime_cron; then
    echo "[OK] 定时任务依赖检查通过（crontab）"
  else
    echo "[WARN] 定时任务依赖缺失：可先使用手动模式（菜单1/5）"
  fi
  return 0
}

ensure_cfg() {
  if [[ ! -f "$CFG" ]]; then
    cat >"$CFG" <<EOF
# 无限续杯配置（Unix 客户端）
# 注意：请勿分享/上传此文件（含密钥）。
SERVER_URL=
USER_KEY=
ACCOUNTS_DIR=$ROOT_DIR/accounts
TARGET_POOL_SIZE=10
TOTAL_HOLD_LIMIT=50
INTERVAL_MINUTES=30
SYNC_TARGET_DIR=
RUN_OUTPUT_MODE=compact
WHAM_PROXY_MODE=auto
WHAM_CONNECT_TIMEOUT=5
WHAM_MAX_TIME=15
TOPUP_CONNECT_TIMEOUT=10
TOPUP_MAX_TIME=180
TOPUP_RETRY=3
TOPUP_RETRY_DELAY=3
PROBE_PARALLEL=6
REFILL_ITER_MAX=6
CLEAN_DELETE_STATUSES=401,429
CLEAN_EXPIRED_DAYS=30
EOF
  fi
}

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

load_cfg() {
  ensure_cfg
  if [[ -f "$CFG" ]]; then
    cfg_tmp="${CFG}.sanitized.$$"
    sanitize_env_to_tmp "$CFG" "$cfg_tmp" 2>/dev/null || cp -f "$CFG" "$cfg_tmp"
    # shellcheck disable=SC1090
    source "$cfg_tmp" || true
    rm -f "$cfg_tmp" >/dev/null 2>&1 || true
  fi
}

sanitize_interval() {
  local v="${1:-30}"
  if ! [[ "$v" =~ ^[0-9]+$ ]]; then
    v=30
  fi
  if (( v < 10 )); then
    echo "10"
  else
    echo "$v"
  fi
}

sanitize_hold() {
  local v="${1:-50}"
  if ! [[ "$v" =~ ^[0-9]+$ ]]; then
    v=50
  fi
  if (( v < 1 )); then
    v=50
  fi
  echo "$v"
}

task_hash() {
  local uk="${1:-}"
  if [[ -z "$uk" ]]; then
    echo "000000"
    return 0
  fi

  if command -v sha256sum >/dev/null 2>&1; then
    printf '%s' "$uk" | sha256sum | awk '{print substr($1,1,6)}'
    return 0
  fi
  if command -v shasum >/dev/null 2>&1; then
    printf '%s' "$uk" | shasum -a 256 | awk '{print substr($1,1,6)}'
    return 0
  fi
  if command -v openssl >/dev/null 2>&1; then
    printf '%s' "$uk" | openssl dgst -sha256 | awk '{print substr($NF,1,6)}'
    return 0
  fi

  echo "000000"
}

ensure_sync_links() {
  local target accounts linked=0 removed=0
  target="${SYNC_TARGET_DIR:-}"
  accounts="${ACCOUNTS_DIR:-$ROOT_DIR/accounts}"
  local manifest
  manifest="$target/.infinite_refill_sync_manifest.txt"

  if [[ -z "$target" ]]; then
    return 0
  fi

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

build_task_entry() {
  cat >"$TASK_ENTRY" <<'EOF'
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
EOF
  chmod +x "$TASK_ENTRY"
}

list_other_refill_blocks() {
  local current_marker="$1"
  local cur
  cur="$(crontab -l 2>/dev/null || true)"
  if [[ -z "$cur" ]]; then
    return 0
  fi
  printf '%s\n' "$cur" | awk -v keep="$current_marker" '
    /^# BEGIN_INFINITE_REFILL_[0-9a-f]{6}$/ { mark=$0; sub(/^# BEGIN_/,"",mark); keep_block=(mark==keep); in_block=1; if(!keep_block){ print mark } next }
    /^# END_INFINITE_REFILL_[0-9a-f]{6}$/ { in_block=0; keep_block=0; next }
  '
}

cleanup_old_tasks() {
  local marker="$1"
  local cur
  cur="$(crontab -l 2>/dev/null || true)"
  if [[ -z "$cur" ]]; then
    return 0
  fi

  local next
  next="$(printf '%s\n' "$cur" | awk -v keep="$marker" '
    BEGIN { skip=0 }
    $0 == "# BEGIN_" keep { skip=1; print; next }
    skip==1 { print; if ($0 == "# END_" keep) { skip=0 } ; next }
    /^# BEGIN_INFINITE_REFILL_[0-9a-f]{6}$/ { skip=1; next }
    /^# END_INFINITE_REFILL_[0-9a-f]{6}$/ { skip=0; next }
    { if(skip==0) print }
  ')"

  printf '%s\n' "$next" | crontab -
}

enable_task() {
  ensure_runtime_base || return 1
  ensure_runtime_cron || return 1
  load_cfg
  local interval hash marker begin end cur filtered cron_line

  INTERVAL_MINUTES="$(sanitize_interval "${INTERVAL_MINUTES:-30}")"
  hash="$(task_hash "${USER_KEY:-}")"
  marker="INFINITE_REFILL_${hash}"
  begin="# BEGIN_${marker}"
  end="# END_${marker}"

  build_task_entry

  cur="$(crontab -l 2>/dev/null || true)"
  filtered="$(printf '%s\n' "$cur" | awk -v b="$begin" -v e="$end" '
    BEGIN { skip=0 }
    $0 == b { skip=1; next }
    $0 == e { skip=0; next }
    { if(skip==0) print }
  ')"

  cron_line="*/${INTERVAL_MINUTES} * * * * bash \"${TASK_ENTRY}\""

  {
    printf '%s\n' "$filtered"
    printf '%s\n' "$begin"
    printf '%s\n' "$cron_line"
    printf '%s\n' "$end"
  } | crontab -

  cleanup_old_tasks "$marker"

  echo "[OK] 已创建/更新定时任务：${REFILL_TASK_PREFIX}_${hash}（每 ${INTERVAL_MINUTES} 分钟）"
  echo "[INFO] 当前 cron 入口：$TASK_ENTRY"

  local current_cron task_line
  current_cron="$(crontab -l 2>/dev/null || true)"
  if printf '%s\n' "$current_cron" | grep -Fq "$begin"; then
    task_line="$(printf '%s\n' "$current_cron" | awk -v b="$begin" '
      $0 == b { in_block=1; next }
      in_block==1 && NF { print; exit }
    ')"
    echo "[OK] 已验证 crontab 已写入任务块：$begin"
    [[ -n "$task_line" ]] && echo "[INFO] 当前任务行：$task_line"
  else
    echo "[WARN] 未在 crontab 中找到对应任务块，请手动执行 crontab -l 检查"
  fi
}

disable_task() {
  local silent="${1:-0}"
  ensure_runtime_cron || return 1
  load_cfg
  local hash marker begin end cur next

  hash="$(task_hash "${USER_KEY:-}")"
  marker="INFINITE_REFILL_${hash}"
  begin="# BEGIN_${marker}"
  end="# END_${marker}"

  cur="$(crontab -l 2>/dev/null || true)"
  if [[ -z "$cur" ]]; then
    [[ "$silent" == "1" ]] || echo "[WARN] 当前用户无 crontab，未发现可关闭任务"
    return 0
  fi

  next="$(printf '%s\n' "$cur" | awk -v b="$begin" -v e="$end" '
    BEGIN { skip=0 }
    $0 == b { skip=1; next }
    $0 == e { skip=0; next }
    { if(skip==0) print }
  ')"

  printf '%s\n' "$next" | crontab -

  if [[ "$silent" != "1" ]]; then
    echo "[OK] 已关闭定时任务：${REFILL_TASK_PREFIX}_${hash}"
  fi
}

save_cfg_interactive() {
  load_cfg

  local default_server_url default_user_key default_accounts_dir
  local detected_sync_dir enable_sync_choice
  local server_url user_key accounts_dir sync_dir
  local interval hold_limit

  default_server_url="${SERVER_URL:-}"
  default_user_key="${USER_KEY:-}"
  default_accounts_dir="${ACCOUNTS_DIR:-$ROOT_DIR/accounts}"
  mkdir -p "$default_accounts_dir"

  read -r -p "请输入服务器地址（填空则使用默认值：${default_server_url}）: " server_url
  server_url="${server_url:-$default_server_url}"

  read -r -p "请输入用户密钥（填空则使用默认值：${default_user_key}）: " user_key
  user_key="${user_key:-$default_user_key}"

  read -r -p "请输入账号文件保存路径（ACCOUNTS_DIR，默认：${default_accounts_dir}）: " accounts_dir
  accounts_dir="${accounts_dir:-$default_accounts_dir}"
  mkdir -p "$accounts_dir"
  accounts_dir="$(cd "$accounts_dir" && pwd)"

  if [[ -d "$HOME/.cli-proxy-api" ]]; then
    detected_sync_dir="$HOME/.cli-proxy-api"
  elif [[ -d "$HOME/cli-proxy-api" ]]; then
    detected_sync_dir="$HOME/cli-proxy-api"
  else
    detected_sync_dir="$HOME/.cli-proxy-api"
  fi

  read -r -p "是否启用同步目录（y/N）: " enable_sync_choice
  if [[ "${enable_sync_choice:-N}" =~ ^[Yy]$ ]]; then
    read -r -p "请输入同步目录（默认：${detected_sync_dir}）: " sync_dir
    sync_dir="${sync_dir:-$detected_sync_dir}"
  else
    sync_dir=""
  fi

  read -r -p "请输入续杯间隔（分钟，最低 10，默认 30）: " interval
  interval="$(sanitize_interval "${interval:-30}")"

  read -r -p "请输入总持有上限 TOTAL_HOLD_LIMIT（默认 50）: " hold_limit
  hold_limit="$(sanitize_hold "${hold_limit:-${TOTAL_HOLD_LIMIT:-50}}")"

  if [[ -z "$server_url" || -z "$user_key" ]]; then
    echo "[ERROR] 服务器地址/用户密钥不能为空"
    return 1
  fi

  cat >"$CFG" <<EOF
# 无限续杯配置（Unix 客户端）
# 注意：请勿分享/上传此文件（含密钥）。
SERVER_URL=$server_url
USER_KEY=$user_key
ACCOUNTS_DIR=$accounts_dir
TARGET_POOL_SIZE=10
TOTAL_HOLD_LIMIT=$hold_limit
INTERVAL_MINUTES=$interval
SYNC_TARGET_DIR=$sync_dir
RUN_OUTPUT_MODE=${RUN_OUTPUT_MODE:-compact}
WHAM_PROXY_MODE=${WHAM_PROXY_MODE:-auto}
WHAM_CONNECT_TIMEOUT=${WHAM_CONNECT_TIMEOUT:-5}
WHAM_MAX_TIME=${WHAM_MAX_TIME:-15}
TOPUP_CONNECT_TIMEOUT=${TOPUP_CONNECT_TIMEOUT:-10}
TOPUP_MAX_TIME=${TOPUP_MAX_TIME:-180}
TOPUP_RETRY=${TOPUP_RETRY:-3}
TOPUP_RETRY_DELAY=${TOPUP_RETRY_DELAY:-3}
PROBE_PARALLEL=${PROBE_PARALLEL:-6}
REFILL_ITER_MAX=${REFILL_ITER_MAX:-6}
CLEAN_DELETE_STATUSES=${CLEAN_DELETE_STATUSES:-401,429}
CLEAN_EXPIRED_DAYS=${CLEAN_EXPIRED_DAYS:-30}
EOF

  echo "[OK] 已保存：$CFG"
  load_cfg
  ensure_sync_links
  return 0
}

has_task_block() {
  local hash marker begin cur
  load_cfg
  hash="$(task_hash "${USER_KEY:-}")"
  marker="INFINITE_REFILL_${hash}"
  begin="# BEGIN_${marker}"
  cur="$(crontab -l 2>/dev/null || true)"
  [[ "$cur" == *"$begin"* ]]
}

reset_task_after_manual() {
  if has_task_block; then
    enable_task >/dev/null 2>&1 || true
    echo "[INFO] 已按手动续杯时间重置下次自动续杯配置"
  fi
}

run_once() {
  set +e
  bash "$SCRIPT_DIR/单次续杯.sh"
  local ec=$?
  set -e

  if [[ "$ec" == "0" ]]; then
    reset_task_after_manual
  elif [[ "$ec" == "4" ]]; then
    echo "[WARN] 服务端已自动禁用当前用户（超日限）。已自动关闭本机定时续杯任务。"
    disable_task 1
  elif [[ "$ec" == "5" ]]; then
    echo "[WARN] 服务端判定滥用并自动封禁。已自动关闭本机定时续杯任务。"
    disable_task 1
  fi

  return "$ec"
}

menu() {
  echo
  echo "====== 无限续杯（配置入口 / macOS/Linux）======"
  echo "配置文件：$CFG"
  echo
  echo "1) 立即执行一次【单次续杯】（使用已保存配置）"
  echo "2) 设置/更新【无限续杯配置】（服务器地址/用户密钥/间隔）"
  echo "3) 开启/更新【定时续杯】cron 任务（单任务串行：先清理后续杯）"
  echo "4) 关闭【定时续杯】cron 任务"
  echo
  echo "5) 同步所有账号（谨慎：高频调用会触发风控）"
  echo "6) 可选：环境诊断（依赖检查）"
  echo "7) 退出"
  echo
}

if [[ -n "$server_url_input" && -n "$user_key_input" ]]; then
  bash "$SCRIPT_DIR/单次续杯.sh" "$server_url_input" "$user_key_input"
  exit 0
fi

if [[ "${1:-}" == "--disable-task-silent" ]]; then
  disable_task 1
  exit 0
fi
if [[ "${1:-}" == "--self-check" ]]; then
  self_check
  exit $?
fi

while true; do
  menu
  read -r -p "请选择 (1-7，默认 3)：" menu_choice
  menu_choice="${menu_choice:-3}"

  case "$menu_choice" in
    1)
      ensure_runtime_base || true
      run_once || true
      ;;
    2)
      save_cfg_interactive || true
      ;;
    3)
      enable_task
      ;;
    4)
      disable_task 0
      ;;
    5)
      ensure_runtime_base || true
      bash "$SCRIPT_DIR/单次续杯.sh" --sync-all
      ;;
    6)
      self_check || true
      ;;
    7)
      exit 0
      ;;
    *)
      echo "[WARN] 无效选择：$menu_choice"
      ;;
  esac
done
