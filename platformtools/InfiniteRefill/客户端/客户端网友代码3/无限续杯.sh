#!/usr/bin/env bash
set -euo pipefail

# 设计约束：把【无限续杯】作为"配置任务入口"（macOS/Linux）。
# - 不带参数：菜单（可输出 cron 配置行 / 单次续杯 / 自动清理）
# - 带参数（服务器地址 用户密钥）：直接执行一次【单次续杯】

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
CFG="$SCRIPT_DIR/无限续杯配置.env"
ROOT_CFG="$ROOT_DIR/无限续杯配置.env"

server_url="${1:-}"
user_key="${2:-}"

if [[ -n "$server_url" && -n "$user_key" ]]; then
  bash "$SCRIPT_DIR/单次续杯.sh" "$server_url" "$user_key"
  exit 0
fi

ensure_cfg() {
  if [[ ! -f "$CFG" ]]; then
    cat >"$CFG" <<'EOF'
# 无限续杯配置（本地文件）
# 注意：请勿分享/上传此文件。
SERVER_URL=
USER_KEY=
ACCOUNTS_DIR=$SCRIPT_DIR/accounts
TARGET_POOL_SIZE=10
TRIGGER_REMAINING=2
INTERVAL_MINUTES=30
AUTO_REFILL_AFTER_CLEAN=1
AUTO_CLEAN_INTERVAL_MINUTES=30
AUTO_CLEAN_APPLY=1
CLEAN_DELETE_STATUSES=401,429
CLEAN_EXPIRED_DAYS=30
SYNC_MODE=none
SYNC_TARGET_DIR=
EOF
  fi
}

load_cfg() {
  ensure_cfg
  if [[ -f "$ROOT_CFG" ]]; then
    # shellcheck disable=SC1090
    source "$ROOT_CFG"
  fi
  # shellcheck disable=SC1090
  source "$CFG"
}

ensure_sync_links() {
  local mode target accounts linked=0 removed=0
  mode="${SYNC_MODE:-none}"
  target="${SYNC_TARGET_DIR:-}"
  accounts="${ACCOUNTS_DIR:-$SCRIPT_DIR/accounts}"
  local manifest
  manifest="$target/.infinite_refill_sync_manifest.txt"

  if [[ "$(printf '%s' "$mode" | tr '[:upper:]' '[:lower:]')" != "symlink" || -z "$target" ]]; then
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

menu() {
  echo
  echo "====== 无限续杯（配置入口 / macOS/Linux）======"
  echo "配置文件：${CFG}"
  echo
  echo "1) 立即执行一次【单次续杯】（使用已保存配置）"
  echo "2) 设置/更新【无限续杯配置】（服务器地址/用户密钥/间隔）"
  echo "3) 生成【定时续杯】cron 配置行（无限续杯依赖自动清理，固定 apply 删除）"
  echo
  echo "4) 同步所有账号（谨慎：高频调用会触发风控）"
  echo "5) 退出"
  echo
}

while true; do
  menu
  read -r -p "请选择 (1-5，默认 3)：" choice
  choice="${choice:-3}"

  case "$choice" in
    1)
      bash "$SCRIPT_DIR/单次续杯.sh"
      ;;
    2)
      load_cfg
      default_server_url="${SERVER_URL:-}"
      default_user_key="${USER_KEY:-}"

      default_accounts_dir="$SCRIPT_DIR/accounts"
      if [[ ! -d "$default_accounts_dir" ]]; then
        for d in "$SCRIPT_DIR"/*; do
          [[ -d "$d/accounts" ]] || continue
          default_accounts_dir="$d/accounts"
          break
        done
      fi
      mkdir -p "$default_accounts_dir"

      read -r -p "请输入服务器地址（填空则使用默认值：${default_server_url}）: " input_server_url
      input_server_url="${input_server_url:-$default_server_url}"
      read -r -p "请输入用户密钥（填空则使用默认值：${default_user_key}）: " input_user_key
      input_user_key="${input_user_key:-$default_user_key}"

      detected_sync_dir=""
      if [[ -d "$HOME/.cli-proxy-api" ]]; then
        detected_sync_dir="$HOME/.cli-proxy-api"
      elif [[ -d "$HOME/cli-proxy-api" ]]; then
        detected_sync_dir="$HOME/cli-proxy-api"
      else
        detected_sync_dir="$HOME/.cli-proxy-api"
      fi
      echo "[INFO] 检测到默认同步目录：${detected_sync_dir}"
      read -r -p "是否同步到CLI目录（y/N）: " do_sync
      if [[ "${do_sync:-N}" =~ ^[Yy]$ ]]; then
        sync_mode="symlink"
        read -r -p "请选择同步目录（填空则使用默认值：${detected_sync_dir}）: " sync_dir
        sync_dir="${sync_dir:-$detected_sync_dir}"
      else
        sync_mode="none"
        sync_dir=""
      fi

      read -r -p "请输入执行间隔（分钟，默认 30）: " interval_min
      interval_min="${interval_min:-30}"
      clean_interval="$interval_min"

      if [[ -z "$input_server_url" || -z "$input_user_key" ]]; then
        echo "[ERROR] 服务器地址/用户密钥不能为空"
        continue
      fi

      cat >"$CFG" <<EOF
# 无限续杯配置（本地文件）
# 注意：请勿分享/上传此文件。
SERVER_URL=$input_server_url
USER_KEY=$input_user_key
ACCOUNTS_DIR=$default_accounts_dir
TARGET_POOL_SIZE=10
TRIGGER_REMAINING=2
INTERVAL_MINUTES=$interval_min
AUTO_REFILL_AFTER_CLEAN=1
AUTO_CLEAN_INTERVAL_MINUTES=$clean_interval
AUTO_CLEAN_APPLY=1
CLEAN_DELETE_STATUSES=401,429
CLEAN_EXPIRED_DAYS=30
SYNC_MODE=$sync_mode
SYNC_TARGET_DIR=$sync_dir
EOF
      echo "[OK] 已保存：$CFG"
      SYNC_MODE="$sync_mode" SYNC_TARGET_DIR="$sync_dir" ACCOUNTS_DIR="$default_accounts_dir" ensure_sync_links
      ;;
    3)
      load_cfg
      interval="${INTERVAL_MINUTES:-30}"
      clean_interval="${AUTO_CLEAN_INTERVAL_MINUTES:-30}"
      script="$SCRIPT_DIR/单次续杯.sh"
      clean="$SCRIPT_DIR/_内部_自动清理.sh"
      ensure_sync_links
      echo
      echo "把下面两行加入 crontab（无限续杯依赖自动清理，固定 apply 删除）："
      echo "*/$clean_interval * * * * bash \"$clean\" apply >/tmp/自动清理.log 2>&1"
      echo "*/$interval * * * * bash \"$script\" >/tmp/无限续杯.log 2>&1"
      ;;
    4)
      bash "$SCRIPT_DIR/单次续杯.sh" --sync-all
      ;;
    5)
      exit 0
      ;;
    *)
      echo "[WARN] 无效选择：${choice}"
      ;;
  esac

done
