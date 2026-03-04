-- 0008_refill_confidence_queue.sql
-- 新增“待置信区 + 二次复现 + 用户不可信度”基础数据结构

-- accounts_v2.owner 约定（增量）：
-- -1 公有池
-- -2 待维修区（401 且可维修）
-- -3 墓地区（永久下线）
-- -4 休眠区（429，等待 eligible_after）
-- -5 待置信区（首次上报先采信后隔离）
-- 其它字符串：私有池持有者 key_hash

-- 待置信区主表：追踪“谁首报、首报原因、复现结果、当前判定状态”
CREATE TABLE IF NOT EXISTS account_confidence_queue (
  account_id TEXT PRIMARY KEY,
  first_reporter_key_hash TEXT NOT NULL,
  first_status_code INTEGER NOT NULL,
  first_reason TEXT,
  first_reported_at TEXT NOT NULL,

  state TEXT NOT NULL DEFAULT 'pending', -- pending | confirmed | rejected
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
CREATE INDEX IF NOT EXISTS idx_account_confidence_queue_updated_at ON account_confidence_queue(updated_at);
CREATE INDEX IF NOT EXISTS idx_account_confidence_queue_reporter ON account_confidence_queue(first_reporter_key_hash);

-- 回放日志：记录每次“待置信号”被下发给哪个用户（用于证据链审计）
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
CREATE INDEX IF NOT EXISTS idx_account_confidence_replays_replayed_to ON account_confidence_replays(replayed_to_key_hash);
CREATE INDEX IF NOT EXISTS idx_account_confidence_replays_replayed_at ON account_confidence_replays(replayed_at);

-- 用户不可信度（日累计；达到阈值可封禁）
CREATE TABLE IF NOT EXISTS user_daily_untrust_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  user_id TEXT NOT NULL,
  day TEXT NOT NULL, -- YYYY-MM-DD (UTC)
  score INTEGER NOT NULL DEFAULT 0,
  updated_at TEXT NOT NULL,
  UNIQUE(user_id, day)
);
CREATE INDEX IF NOT EXISTS idx_user_daily_untrust_events_user_day ON user_daily_untrust_events(user_id, day);

-- 封禁审计（封禁后不自动解封）
CREATE TABLE IF NOT EXISTS user_ban_audit (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  user_id TEXT NOT NULL,
  reason TEXT NOT NULL,
  detail TEXT,
  created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_user_ban_audit_user_id ON user_ban_audit(user_id);
CREATE INDEX IF NOT EXISTS idx_user_ban_audit_created_at ON user_ban_audit(created_at);
