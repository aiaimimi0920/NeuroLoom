-- 0003_client_activity.sql
-- 引入“客户端活跃度”表：用于实现
-- “如果用户 7 天都没有对平台发起任何请求，则视为离开；其已领取作品解绑回公共池”。
--
-- 说明：本迁移是幂等的（CREATE TABLE IF NOT EXISTS）。

CREATE TABLE IF NOT EXISTS client_activity (
  key_hash TEXT PRIMARY KEY,
  role TEXT NOT NULL, -- user | upload
  last_seen_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_client_activity_last_seen_at ON client_activity(last_seen_at);
