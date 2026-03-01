-- 0004_accounts_auth_json.sql
-- 为旧“续杯账号 JSON”链路打通 auth_json 加密入库：
-- - 新增 accounts.has_auth_json 用于快速过滤（避免对大 TEXT 扫描）
-- - 增加索引优化 /v1/refill/topup 下发查询

-- 已经执行过的环境重复执行可能报错：这里用 try/catch 的思路拆分（D1/SQLite 不支持 IF NOT EXISTS on ALTER COLUMN）。
-- Wrangler/D1 迁移通常只会跑一次；这里仍保持简单。

ALTER TABLE accounts ADD COLUMN has_auth_json INTEGER NOT NULL DEFAULT 0;

-- 兼容旧数据：如果历史上写入过明文/其它形式的 auth_json，这里只做“非空”标记。
-- 新代码会把 auth_json 写成 AES-GCM(base64) 加密串。
UPDATE accounts
SET has_auth_json = 1
WHERE auth_json IS NOT NULL AND LENGTH(TRIM(auth_json)) > 0;

CREATE INDEX IF NOT EXISTS idx_accounts_invalid_has_auth_last_seen
ON accounts(invalid, has_auth_json, last_seen_at);
