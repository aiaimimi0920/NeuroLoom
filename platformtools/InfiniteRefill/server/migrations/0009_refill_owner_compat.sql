-- 0009_refill_owner_compat.sql
-- v2 owner 负值语义兼容迁移：
-- 旧语义：-4 表示“维修中”
-- 新语义：-4 表示“休眠区(429)”，-5 表示“待置信区”
--
-- 迁移策略：
-- 1) 仅做最小结构补齐（容错）
-- 2) 历史 -4 账号若不存在休眠记录，则回收到公有池(-1)，避免被误当作429休眠
-- 3) 若存在 account_v2_sleep 记录，则保留 -4 语义（视为休眠）

CREATE TABLE IF NOT EXISTS account_v2_sleep (
  account_id TEXT PRIMARY KEY,
  eligible_after TEXT NOT NULL,
  last_status_code INTEGER,
  updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_account_v2_sleep_eligible_after ON account_v2_sleep(eligible_after);

-- 历史兼容：把“没有 sleep 记录”的 -4 释放回公有池
UPDATE accounts_v2
SET owner='-1', updated_at=strftime('%Y-%m-%dT%H:%M:%SZ','now')
WHERE owner='-4'
  AND account_id NOT IN (SELECT account_id FROM account_v2_sleep);

-- 待置信区相关结构（幂等补齐，便于直接迁移）
CREATE TABLE IF NOT EXISTS account_confidence_queue (
  account_id TEXT PRIMARY KEY,
  first_reporter_key_hash TEXT NOT NULL,
  first_status_code INTEGER NOT NULL,
  first_reason TEXT,
  first_reported_at TEXT NOT NULL,
  state TEXT NOT NULL DEFAULT 'pending',
  replay_count INTEGER NOT NULL DEFAULT 0,
  confirm_count INTEGER NOT NULL DEFAULT 0,
  reject_count INTEGER NOT NULL DEFAULT 0,
  last_replayed_to_key_hash TEXT,
  last_replayed_at TEXT,
  last_feedback_key_hash TEXT,
  last_feedback_status_code INTEGER,
  last_feedback_note TEXT,
  last_feedback_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_account_confidence_queue_state ON account_confidence_queue(state);

CREATE TABLE IF NOT EXISTS account_confidence_replays (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  account_id TEXT NOT NULL,
  replayed_to_key_hash TEXT NOT NULL,
  replayed_at TEXT NOT NULL,
  source_reporter_key_hash TEXT,
  source_status_code INTEGER,
  source_reason TEXT
);
CREATE INDEX IF NOT EXISTS idx_account_confidence_replays_account_id ON account_confidence_replays(account_id);

CREATE TABLE IF NOT EXISTS user_daily_untrust_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  user_id TEXT NOT NULL,
  day TEXT NOT NULL,
  score INTEGER NOT NULL DEFAULT 0,
  updated_at TEXT NOT NULL,
  UNIQUE(user_id, day)
);

CREATE TABLE IF NOT EXISTS user_ban_audit (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  user_id TEXT NOT NULL,
  reason TEXT NOT NULL,
  detail TEXT,
  created_at TEXT NOT NULL
);
