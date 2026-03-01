#!/usr/bin/env bash
set -euo pipefail

# 设计约束：把【无限续杯】作为“配置任务入口”（macOS/Linux）。
# - 不带参数：菜单（可输出 cron 配置行 / 单次续杯 / 自动清理）
# - 带参数（服务器地址 用户密钥）：直接执行一次【单次续杯】

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CFG="$SCRIPT_DIR/状态/无限续杯配置.env"

服务器地址="${1:-}"
用户密钥="${2:-}"

if [[ -n "$服务器地址" && -n "$用户密钥" ]]; then
  bash "$SCRIPT_DIR/单次续杯.sh" "$服务器地址" "$用户密钥"
  exit 0
fi

ensure_cfg() {
  mkdir -p "$SCRIPT_DIR/状态"
  if [[ ! -f "$CFG" ]]; then
    cat >"$CFG" <<'EOF'
# 无限续杯配置（本地文件）
# 注意：请勿分享/上传此文件。
SERVER_URL=
USER_KEY=
INTERVAL_MINUTES=30
AUTO_CLEAN_INTERVAL_MINUTES=30
AUTO_CLEAN_APPLY=0
EOF
  fi
}

load_cfg() {
  ensure_cfg
  # shellcheck disable=SC1090
  source "$CFG"
}

menu() {
  echo
  echo "====== 无限续杯（配置入口 / macOS/Linux）======"
  echo "配置文件：$CFG"
  echo
  echo "1) 立即执行一次【单次续杯】（使用已保存配置）"
  echo "2) 设置/更新【无限续杯配置】（服务器地址/用户密钥/间隔）"
  echo "3) 生成【定时续杯】cron 配置行（从配置读取，不把密钥写进 crontab）"
  echo
  echo "4) 自动清理（仅删 401）：DryRun（不删除）"
  echo "5) 自动清理（仅删 401）：Apply（执行删除）"
  echo "6) 生成【自动清理】cron 配置行"
  echo
  echo "7) 退出"
  echo
}

while true; do
  menu
  read -r -p "请选择 (1-7，默认 3)：" 选择
  选择="${选择:-3}"

  case "$选择" in
    1)
      bash "$SCRIPT_DIR/单次续杯.sh"
      ;;
    2)
      ensure_cfg
      read -r -p "请输入服务器地址（例如 https://127.0.0.1:8787 ）: " 服务器地址
      read -r -p "请输入用户密钥（USER_KEY 或 UPLOAD_KEY）: " 用户密钥
      read -r -p "请输入续杯间隔（分钟，默认 30）: " 间隔分钟
      间隔分钟="${间隔分钟:-30}"

      if [[ -z "$服务器地址" || -z "$用户密钥" ]]; then
        echo "[ERROR] 服务器地址/用户密钥不能为空"
        continue
      fi

      cat >"$CFG" <<EOF
# 无限续杯配置（本地文件）
# 注意：请勿分享/上传此文件。
SERVER_URL=$服务器地址
USER_KEY=$用户密钥
INTERVAL_MINUTES=$间隔分钟
AUTO_CLEAN_INTERVAL_MINUTES=30
AUTO_CLEAN_APPLY=0
EOF
      echo "[OK] 已保存：$CFG"
      ;;
    3)
      load_cfg
      interval="${INTERVAL_MINUTES:-30}"
      script="$SCRIPT_DIR/单次续杯.sh"
      echo
      echo "把下面这一行加入 crontab 即可（crontab -e）："
      echo "*/$interval * * * * bash \"$script\" >/tmp/无限续杯.log 2>&1"
      ;;
    4)
      bash "$SCRIPT_DIR/自动清理/一键清理_仅删401.sh"
      ;;
    5)
      bash "$SCRIPT_DIR/自动清理/一键清理_仅删401.sh" apply
      ;;
    6)
      load_cfg
      interval="${AUTO_CLEAN_INTERVAL_MINUTES:-30}"
      mode="${AUTO_CLEAN_APPLY:-0}"
      clean="$SCRIPT_DIR/自动清理/一键清理_仅删401.sh"
      echo
      echo "把下面这一行加入 crontab 即可（crontab -e）："
      if [[ "$mode" == "1" ]]; then
        echo "*/$interval * * * * bash \"$clean\" apply >/tmp/自动清理.log 2>&1"
      else
        echo "*/$interval * * * * bash \"$clean\" >/tmp/自动清理.log 2>&1"
      fi
      ;;
    7)
      exit 0
      ;;
    *)
      echo "[WARN] 无效选择：$选择"
      ;;
  esac

done
