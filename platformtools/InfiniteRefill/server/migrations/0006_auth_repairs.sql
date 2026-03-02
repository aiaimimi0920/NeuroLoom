-- 0006_auth_repairs.sql
-- 引入“账号修缮（auth repair）失败计数/墓地区”机制。
-- 目的：当 repairer 修补 auth json 失败时，仅上报 account_id + note，
--       服务端累计 3 次后将其判定进入墓地区（不再重试）。

CREATE TABLE IF NOT EXISTS auth_repairs (
  account_id TEXT PRIMARY KEY,
  fail_count INTEGER NOT NULL DEFAULT 0,
  first_failed_at TEXT NOT NULL,
  last_failed_at TEXT NOT NULL,
  last_fail_note TEXT
);

CREATE TABLE IF NOT EXISTS auth_tombstones (
  account_id TEXT PRIMARY KEY,
  reason TEXT,
  note TEXT,
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_auth_repairs_last_failed_at ON auth_repairs(last_failed_at);
