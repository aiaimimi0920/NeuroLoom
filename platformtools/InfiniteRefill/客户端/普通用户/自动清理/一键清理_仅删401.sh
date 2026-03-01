#!/usr/bin/env bash
set -euo pipefail

# 一键直连清理（仅删 401）
# - 从 ../../../config.yaml 读取 auth-dir
# - 直连探测 https://chatgpt.com/backend-api/wham/usage
# - 仅当 HTTP 401 才会删除
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

if [[ ! -f "$CFG" ]]; then
  echo "[ERROR] 找不到 config.yaml：$CFG"
  exit 3
fi

# 粗略解析 auth-dir（支持：auth-dir: \"...\" 或 auth-dir: ...）
ACCOUNTS_DIR="$(grep -E '^[[:space:]]*auth-dir[[:space:]]*:' -m 1 "$CFG" | sed -E 's/^[[:space:]]*auth-dir[[:space:]]*:[[:space:]]*//; s/#.*$//; s/^["\x27]//; s/["\x27][[:space:]]*$//')"

if [[ -z "$ACCOUNTS_DIR" ]]; then
  echo "[ERROR] 无法从 $CFG 解析 auth-dir"
  exit 3
fi

OUT_DIR="$SCRIPT_DIR/out/清理-401-$(date +%Y%m%d-%H%M%S)"
PLAN="$OUT_DIR/计划删除-401.txt"
REPORT="$OUT_DIR/报告.txt"
BACKUP="$OUT_DIR/backup"

mkdir -p "$BACKUP"
: > "$PLAN"
: > "$REPORT"

echo "Config:      $CFG" | tee -a "$REPORT"
echo "AccountsDir: $ACCOUNTS_DIR" | tee -a "$REPORT"
if [[ "$APPLY" == "1" ]]; then
  echo "Apply:      true（会删除 401 文件；会先备份）" | tee -a "$REPORT"
else
  echo "Apply:      false（DryRun：只生成计划，不删除）" | tee -a "$REPORT"
fi

echo >> "$REPORT"

total=0
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

  if [[ -n "$local_aid" ]]; then
    status="$(curl -sS -o /dev/null -w "%{http_code}" \
      -H "Authorization: Bearer $local_token" \
      -H "Chatgpt-Account-Id: $local_aid" \
      -H "Accept: application/json, text/plain, */*" \
      -H "User-Agent: codex_cli_rs/0.76.0 (macOS/Linux)" \
      "https://chatgpt.com/backend-api/wham/usage" || true)"
  else
    status="$(curl -sS -o /dev/null -w "%{http_code}" \
      -H "Authorization: Bearer $local_token" \
      -H "Accept: application/json, text/plain, */*" \
      -H "User-Agent: codex_cli_rs/0.76.0 (macOS/Linux)" \
      "https://chatgpt.com/backend-api/wham/usage" || true)"
  fi

  echo "[PROBE] $(basename "$f") status=$status" >> "$REPORT"

  if [[ "$status" == "401" ]]; then
    cand=$((cand+1))
    echo "$f" >> "$PLAN"

    if [[ "$APPLY" == "1" ]]; then
      cp -f "$f" "$BACKUP/$(basename "$f")" 2>/dev/null || true
      rm -f "$f" 2>/dev/null || true
      if [[ ! -f "$f" ]]; then
        deleted=$((deleted+1))
        echo "[DEL]  $(basename "$f")" >> "$REPORT"
      else
        echo "[FAIL] $(basename "$f")" >> "$REPORT"
      fi
    fi
  fi

done

{
  echo
  echo "[DONE] total=$total candidates_401=$cand deleted=$deleted"
  echo "计划文件：$PLAN"
  echo "报告文件：$REPORT"
} | tee -a "$REPORT"

# 清理后自动补齐（可选）：如果实际删除了文件且开启了自动补齐，则触发续杯请求
if [[ "$APPLY" == "1" && "$deleted" -gt 0 ]]; then
  CFG_USER="$SCRIPT_DIR/../状态/无限续杯配置.env"
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
    echo "[INFO] 清理后剩余=$remain 目标=$target：缺口=$missing" | tee -a "$REPORT"

    if [[ "$auto_refill" != "1" ]]; then
      echo "[INFO] 未开启自动补齐（AUTO_REFILL_AFTER_CLEAN=1 才会自动续杯）" | tee -a "$REPORT"
      exit 0
    fi

    if [[ -z "${SERVER_URL:-}" || -z "${USER_KEY:-}" ]]; then
      echo "[WARN] 未配置 SERVER_URL/USER_KEY，无法自动补齐" | tee -a "$REPORT"
      exit 0
    fi

    OUT_REFILL="$SCRIPT_DIR/../状态/清理后补齐结果.jsonl"

    body="$SCRIPT_DIR/../状态/_topup_body.json"
    echo "{\"target_pool_size\":$target,\"reports\":[]}" > "$body"

    echo "[INFO] 开始补齐：请求一次 topup（目标=$target），追加写入：$OUT_REFILL" | tee -a "$REPORT"

    resp="$(curl -sS -X POST "$SERVER_URL/v1/refill/topup" \
      -H "X-User-Key: $USER_KEY" \
      -H "Content-Type: application/json" \
      --data-binary "@$body" || true)"
    echo "$resp" >> "$OUT_REFILL"

    echo "[OK] 已触发补齐请求" | tee -a "$REPORT"
  fi
fi
