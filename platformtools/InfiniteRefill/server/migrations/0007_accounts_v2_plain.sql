-- 0007_accounts_v2_plain.sql
-- v2: 新项目明文账号模型（放弃旧 email_hash/auth_json 加密链路）

-- 账号池：明文结构 + owner 状态语义
-- owner:
--   -1 公有池
--   -2 维修池
--   -3 墓地
--   -4 维修中
--   其它字符串：私有池持有者 key_hash
CREATE TABLE IF NOT EXISTS accounts_v2 (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  account_id TEXT NOT NULL UNIQUE,
  email TEXT NOT NULL,
  password TEXT NOT NULL,
  r2_url TEXT,
  owner TEXT NOT NULL DEFAULT '-1',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  last_seen_at TEXT,
  last_refilled_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_accounts_v2_owner ON accounts_v2(owner);
CREATE INDEX IF NOT EXISTS idx_accounts_v2_updated_at ON accounts_v2(updated_at);

-- 用户主表：服务端生成唯一 user_id
CREATE TABLE IF NOT EXISTS users_v2 (
  id TEXT PRIMARY KEY,
  display_name TEXT,
  roles TEXT NOT NULL DEFAULT '', -- 逗号分隔: super_admin,admin,uploader,user,repairer
  current_account_ids TEXT NOT NULL DEFAULT '[]', -- JSON array
  daily_refill_limit INTEGER NOT NULL DEFAULT 200,
  disabled INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_users_v2_disabled ON users_v2(disabled);

-- key 与 user 绑定（沿用现有 key_hash 鉴权体系）
CREATE TABLE IF NOT EXISTS user_keys_v2 (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  key_hash TEXT NOT NULL UNIQUE,
  user_id TEXT NOT NULL,
  role TEXT NOT NULL, -- super_admin | admin | uploader | user | repairer
  enabled INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_user_keys_v2_user_id ON user_keys_v2(user_id);
CREATE INDEX IF NOT EXISTS idx_user_keys_v2_role ON user_keys_v2(role);

-- 用户每日续杯计数（反滥用）
CREATE TABLE IF NOT EXISTS user_daily_refill_usage (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  user_id TEXT NOT NULL,
  day TEXT NOT NULL, -- YYYY-MM-DD (UTC)
  refilled_count INTEGER NOT NULL DEFAULT 0,
  updated_at TEXT NOT NULL,
  UNIQUE(user_id, day)
);
CREATE INDEX IF NOT EXISTS idx_user_daily_refill_usage_user_day ON user_daily_refill_usage(user_id, day);
