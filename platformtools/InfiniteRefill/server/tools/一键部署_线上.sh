#!/usr/bin/env sh
set -eu

# 一键部署（线上）：不在文件里写任何密钥；所有敏感值通过 wrangler secret put 交互输入。

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
cd "$ROOT"

echo "[STEP] 1/4 设置必须的 secrets（按提示逐个粘贴，不会回显）"
	npx wrangler secret put ADMIN_TOKEN
	npx wrangler secret put REFILL_KEYS_MASTER_KEY_B64
	npx wrangler secret put ACCOUNTS_MASTER_KEY_B64

echo "[STEP] 2/4 设置 R2 S3 API token（用于 presigned URL；按提示逐个粘贴，不会回显）"
	npx wrangler secret put R2_ACCESS_KEY_ID
	npx wrangler secret put R2_SECRET_ACCESS_KEY

echo "[STEP] 3/4 初始化/更新远端 D1 schema"
	# 0001_init 仅保证“全新库”具备最新 schema；已有库仍需跑增量迁移（ALTER TABLE）。
	npx wrangler d1 execute refill_server_v2 --remote --file ./migrations/0001_init.sql
	npx wrangler d1 execute refill_server_v2 --remote --file ./migrations/0004_accounts_auth_json.sql
	npx wrangler d1 execute refill_server_v2 --remote --file ./migrations/0005_topup_issues.sql

echo "[STEP] 4/4 部署 Worker"
	npx wrangler deploy

echo "[OK] 已完成。"
