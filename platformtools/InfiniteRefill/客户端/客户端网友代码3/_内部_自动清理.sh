#!/usr/bin/env bash
set -euo pipefail

# 一键直连清理（可配置删 401/429 + 过期文件）
# - 仅读取本地 accounts 目录
# - 直连探测 https://chatgpt.com/backend-api/wham/usage
# - 可按 CLEAN_DELETE_STATUSES 删除状态码命中项（默认 401,429）
# - 可按 CLEAN_EXPIRED_DAYS 删除“过期文件”（按文件修改时间）
# - 默认：DryRun（不删除，只生成计划与报告）
#
# 依赖：curl
# JSON 解析（任选其一）：python3（推荐）/ osascript(JXA, macOS 自带) / jq（可选兜底）

APPLY=0
if [[ "${1:-}" == "apply" || "${2:-}" == "apply" || "${3:-}" == "apply" ]]; then
  APPLY=1
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../../.." && pwd)"

LIB_SH="$ROOT_DIR/客户端/_lib/json.sh"
# shellcheck disable=SC1090
source "$LIB_SH"

need_cmd curl
need_json_parser

CFG="$ROOT_DIR/config.yaml"
USER_CFG="$SCRIPT_DIR/无限续杯配置.env"
ROOT_CFG="$ROOT_DIR/无限续杯配置.env"

ACCOUNTS_DIR="$SCRIPT_DIR/accounts"
CLEAN_DELETE_STATUSES="401,429"
CLEAN_EXPIRED_DAYS="30"
if [[ -f "$ROOT_CFG" ]]; then
  # shellcheck disable=SC1090
  source "$ROOT_CFG" || true
fi
if [[ -f "$USER_CFG" ]]; then
  # shellcheck disable=SC1090
  source "$USER_CFG" || true
fi

status_in_list() {
  local code="$1"
  local csv="${CLEAN_DELETE_STATUSES:-401,429,403}"
  local norm
  norm=",$(printf "%s" "$csv" | tr ' ' ',' | sed -E 's/,+/,/g; s/^,+//; s/,+$//'),"
  [[ "$norm" == *",$code,"* ]]
}

file_mtime_epoch() {
  local f="$1"
  if stat -c %Y "$f" >/dev/null 2>&1; then
    stat -c %Y "$f"
    return 0
  fi
  if stat -f %m "$f" >/dev/null 2>&1; then
    stat -f %m "$f"
    return 0
  fi
  return 1
}

mkdir -p "$ACCOUNTS_DIR"
if [[ ! -d "$ACCOUNTS_DIR" ]]; then
  echo "[ERROR] 账户目录不存在且创建失败：$ACCOUNTS_DIR"
  exit 3
fi

OUT_DIR="$SCRIPT_DIR/out/清理-401-$(date +%Y%m%d-%H%M%S)"
PLAN="$OUT_DIR/计划删除-401.txt"
REPORT="$OUT_DIR/报告.txt"
BACKUP="$OUT_DIR/backup"

mkdir -p "$BACKUP"
: > "$PLAN"
: > "$REPORT"

echo "Config:      $USER_CFG" | tee -a "$REPORT"
echo "AccountsDir: $ACCOUNTS_DIR" | tee -a "$REPORT"
echo "DeleteStatus: ${CLEAN_DELETE_STATUSES:-401,429}" | tee -a "$REPORT"
echo "ExpiredDays: ${CLEAN_EXPIRED_DAYS:-30}" | tee -a "$REPORT"
if [[ "$APPLY" == "1" ]]; then
  echo "Apply:      true（会删除命中状态/过期文件；会先备份）" | tee -a "$REPORT"
else
  echo "Apply:      false（DryRun：只生成计划，不删除）" | tee -a "$REPORT"
fi

echo >> "$REPORT"

total=0
probed_ok=0
net_fail=0
cand=0
deleted=0

shopt -s nullglob
for f in "$ACCOUNTS_DIR"/*.json; do
  total=$((total+1))

  local_type=""
  local_token=""
  local_aid=""
  {
    IFS= read -r local_type || true
    IFS= read -r local_token || true
    IFS= read -r local_aid || true
    IFS= read -r _ || true
  } < <(json_auth_fields4 "$f" 2>/dev/null || true)

  if [[ "$local_type" != "codex" ]]; then
    echo "[SKIP] $(basename "$f") (type=$local_type)" >> "$REPORT"
    continue
  fi

  if [[ -z "$local_token" ]]; then
    echo "[SKIP] $(basename "$f") (missing access_token)" >> "$REPORT"
    continue
  fi

  wham_body="$OUT_DIR/.wham.clean.$$.tmp"
  if [[ -n "$local_aid" ]]; then
    status="$(curl -sS -o "$wham_body" -w "%{http_code}" \
      -H "Authorization: Bearer $local_token" \
      -H "Chatgpt-Account-Id: $local_aid" \
      -H "Accept: application/json, text/plain, */*" \
      -H "User-Agent: codex_cli_rs/0.76.0 (macOS/Linux)" \
      "https://chatgpt.com/backend-api/wham/usage" || true)"
  else
    status="$(curl -sS -o "$wham_body" -w "%{http_code}" \
      -H "Authorization: Bearer $local_token" \
      -H "Accept: application/json, text/plain, */*" \
      -H "User-Agent: codex_cli_rs/0.76.0 (macOS/Linux)" \
      "https://chatgpt.com/backend-api/wham/usage" || true)"
  fi

  status="$(json_normalize_wham_status "$status" "$wham_body")"
  rm -f "$wham_body" 2>/dev/null || true

  if ! [[ "$status" =~ ^[0-9]{3}$ ]] || [[ "$status" == "000" ]]; then
    net_fail=$((net_fail+1))
    echo "[NETFAIL] $(basename "$f") status=000" >> "$REPORT"
    continue
  fi
  probed_ok=$((probed_ok+1))

  echo "[PROBE] $(basename "$f") status=$status" >> "$REPORT"

  delete_reason=""
  if status_in_list "$status"; then
    delete_reason="status=$status"
  fi

  exp_days="${CLEAN_EXPIRED_DAYS:-30}"
  if [[ "$exp_days" =~ ^[0-9]+$ ]] && [[ "$exp_days" -gt 0 ]]; then
    mtime="$(file_mtime_epoch "$f" || echo 0)"
    now_epoch="$(date +%s)"
    if [[ "$mtime" =~ ^[0-9]+$ ]] && [[ "$mtime" -gt 0 ]]; then
      age_days=$(( (now_epoch - mtime) / 86400 ))
      if [[ "$age_days" -ge "$exp_days" ]]; then
        if [[ -n "$delete_reason" ]]; then
          delete_reason="$delete_reason,expired=${age_days}d"
        else
          delete_reason="expired=${age_days}d"
        fi
      fi
    fi
  fi

  if [[ -n "$delete_reason" ]]; then
    cand=$((cand+1))
    echo "$f # $delete_reason" >> "$PLAN"

    if [[ "$APPLY" == "1" ]]; then
      cp -f "$f" "$BACKUP/$(basename "$f")" 2>/dev/null || true
      rm -f "$f" 2>/dev/null || true
      if [[ ! -f "$f" ]]; then
        deleted=$((deleted+1))
        echo "[DEL]  $(basename "$f") reason=$delete_reason" >> "$REPORT"
      else
        echo "[FAIL] $(basename "$f") reason=$delete_reason" >> "$REPORT"
      fi
    fi
  fi

done

{
  echo
  echo "[DONE] total=$total probed_ok=$probed_ok net_fail=$net_fail candidates_matched=$cand deleted=$deleted"
  echo "计划文件：$PLAN"
  echo "报告文件：$REPORT"
} | tee -a "$REPORT"

# 清理后自动补齐（可选）：如果实际删除了文件且开启了自动补齐，则触发续杯请求
if [[ "$APPLY" == "1" && "$deleted" -gt 0 ]]; then
  CFG_USER="$SCRIPT_DIR/无限续杯配置.env"
  if [[ -f "$CFG_USER" ]]; then
    # shellcheck disable=SC1090
    source "$CFG_USER"
  fi

  target="${TARGET_POOL_SIZE:-10}"
  auto_refill="${AUTO_REFILL_AFTER_CLEAN:-0}"

  # 统计当前 codex 文件数量（用 json_auth_fields4，避免 jq 依赖）
  remain=0
  for ff in "$ACCOUNTS_DIR"/*.json; do
    [[ -f "$ff" ]] || continue
    t=""
    { IFS= read -r t || true; } < <(json_auth_fields4 "$ff" 2>/dev/null || true)
    [[ "$t" == "codex" ]] && remain=$((remain+1))
  done

  missing=$((target - remain))
  if [[ "$missing" -gt 0 ]]; then
    echo "[INFO] 清理后剩余=$remain 目标=${target}：缺口=$missing" | tee -a "$REPORT"

    if [[ "$auto_refill" != "1" ]]; then
      echo "[INFO] 未开启自动补齐（AUTO_REFILL_AFTER_CLEAN=1 才会自动续杯）" | tee -a "$REPORT"
      exit 0
    fi

    if [[ -z "${SERVER_URL:-}" || -z "${USER_KEY:-}" ]]; then
      echo "[WARN] 未配置 SERVER_URL/USER_KEY，无法自动补齐" | tee -a "$REPORT"
      exit 0
    fi

    OUT_REFILL="$SCRIPT_DIR/out/清理后补齐结果.jsonl"

    body="$SCRIPT_DIR/out/_topup_body.json"
    echo "{\"target_pool_size\":$target,\"reports\":[]}" > "$body"

    echo "[INFO] 开始补齐：请求一次 topup（目标=${target}），追加写入：${OUT_REFILL}" | tee -a "$REPORT"

    resp="$(curl -sS -X POST "$SERVER_URL/v1/refill/topup" \
      -H "X-User-Key: $USER_KEY" \
      -H "Content-Type: application/json" \
      --data-binary "@$body" || true)"
    echo "$resp" >> "$OUT_REFILL"

    echo "[OK] 已触发补齐请求" | tee -a "$REPORT"
  fi
fi
