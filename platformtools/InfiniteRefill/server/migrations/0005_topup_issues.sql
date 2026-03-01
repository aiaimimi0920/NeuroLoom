-- 0005_topup_issues.sql
-- 记录 /v1/refill/topup 的下发审计，用于：
-- 1) 统计（/admin/stats）
-- 2) 软性避免短时间重复下发同一账号（soft lease）

CREATE TABLE IF NOT EXISTS topup_issues (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  email_hash TEXT NOT NULL,
  issued_at TEXT NOT NULL,
  issued_to_key_hash TEXT NOT NULL,
  issued_to_role TEXT NOT NULL,
  request_received_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_topup_issues_issued_at ON topup_issues(issued_at);
CREATE INDEX IF NOT EXISTS idx_topup_issues_email_hash_issued_at ON topup_issues(email_hash, issued_at);
