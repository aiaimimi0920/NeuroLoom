-- 0001_init.sql
-- 初始化全量 schema（等同于 server/schema.sql 的效果；用于“无历史包袱”的一把梭初始化）

-- Upload keys：普通管理员凭据（只存 hash）
-- 说明：普通管理员只能做“上报/注册”，不能 claim refill key，也不能访问 /admin/*。
CREATE TABLE IF NOT EXISTS upload_keys (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  key_hash TEXT NOT NULL UNIQUE,
  label TEXT,
  enabled INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL
);

-- User keys：普通用户凭据（只存 hash）
-- 说明：普通用户只能 claim refill key。
CREATE TABLE IF NOT EXISTS user_keys (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  key_hash TEXT NOT NULL UNIQUE,
  label TEXT,
  enabled INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL
);

-- Refill keys：给客户端“无限续杯”的 key（加密存储，便于分发/追踪）
CREATE TABLE IF NOT EXISTS refill_keys (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  key_hash TEXT NOT NULL UNIQUE,
  key_enc_b64 TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'available', -- available | claimed | revoked

  -- 兼容字段：旧版曾用 claimed_by_upload_key_hash
  claimed_by_upload_key_hash TEXT,

  -- 新字段：由“普通用户”领取时记录
  claimed_by_user_key_hash TEXT,

  claimed_at TEXT,
  created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_refill_keys_status ON refill_keys(status);

-- accounts：记录识别信息和 token 数据
-- - auth_json：敏感字段。仅用于旧 /v1/refill/topup 链路下发。
--   服务端存储为 AES-GCM(base64) 加密串（需要 ACCOUNTS_MASTER_KEY_B64）。
-- - has_auth_json：快速过滤字段，避免对大 TEXT 做 LENGTH/TRIM 扫描。
CREATE TABLE IF NOT EXISTS accounts (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  email_hash TEXT NOT NULL,
  account_id TEXT,
  first_seen_at TEXT NOT NULL,
  last_seen_at TEXT NOT NULL,
  last_status_code INTEGER,
  last_probed_at TEXT,
  invalid INTEGER NOT NULL DEFAULT 0,
  invalid_at TEXT,
  has_auth_json INTEGER NOT NULL DEFAULT 0,
  access_token TEXT,
  refresh_token TEXT,
  auth_json TEXT,
  UNIQUE(email_hash)
);
CREATE INDEX IF NOT EXISTS idx_accounts_invalid ON accounts(invalid);
CREATE INDEX IF NOT EXISTS idx_accounts_invalid_has_auth_last_seen ON accounts(invalid, has_auth_json, last_seen_at);

-- topup_issues：记录 /v1/refill/topup 下发审计（用于避免短时间重复下发同一账号）
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

-- probe 上报明细（审计/统计）
CREATE TABLE IF NOT EXISTS probes (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  upload_key_hash TEXT NOT NULL,
  email_hash TEXT NOT NULL,
  account_id TEXT,
  status_code INTEGER,
  probed_at TEXT NOT NULL,
  received_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_probes_received_at ON probes(received_at);

-- invalid 去重库（用于“上传时/续杯时去重”，这里只记录身份，不记录 token）
CREATE TABLE IF NOT EXISTS invalid_accounts (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  email_hash TEXT NOT NULL UNIQUE,
  account_id TEXT,
  first_invalid_at TEXT NOT NULL,
  last_invalid_at TEXT NOT NULL,
  last_status_code INTEGER
);

-- quota exhausted 去重库（用于“周限额暂存”）
CREATE TABLE IF NOT EXISTS exhausted_accounts (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  email_hash TEXT NOT NULL UNIQUE,
  account_id TEXT,
  exhausted_at TEXT NOT NULL,
  last_status_code INTEGER,
  last_probed_at TEXT,
  eligible_after TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_exhausted_eligible ON exhausted_accounts(eligible_after);

-- pending_verification_accounts：当服务端（如 Cloudflare）因 403 被限制等网络错误无法进行二次校验时，
-- 有限信任客户端将其标记为无效，同时送入此列表等待后续挂代理的专门验证节点拾取做真正的二次校验。
CREATE TABLE IF NOT EXISTS pending_verification_accounts (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  email_hash TEXT NOT NULL UNIQUE,
  account_id TEXT,
  reported_status_code INTEGER,
  reported_at TEXT NOT NULL
);

-- packages_batches：手动分发包批次（仅存“对象索引”，不保存明文 key）
CREATE TABLE IF NOT EXISTS package_batches (
  batch_id TEXT PRIMARY KEY,
  key_type TEXT NOT NULL, -- user | upload
  count INTEGER NOT NULL,
  label TEXT,
  bind_pool_size INTEGER,
  server_url TEXT,
  created_at TEXT NOT NULL,
  expires_at TEXT NOT NULL
);

-- package_objects：每个批次里的 R2 对象键（zip 与分发清单）
CREATE TABLE IF NOT EXISTS package_objects (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  batch_id TEXT NOT NULL,
  name TEXT NOT NULL,      -- 例如 1.zip / 分发清单.txt
  kind TEXT NOT NULL,      -- zip | manifest
  r2_key TEXT NOT NULL,    -- 例如 batches/<batch_id>/1.zip
  created_at TEXT NOT NULL,
  UNIQUE(batch_id, name)
);
CREATE INDEX IF NOT EXISTS idx_package_objects_batch_id ON package_objects(batch_id);

-- artworks：艺术品索引（正文存 R2，D1 只存索引）
-- 区域/状态机：
-- - available：公有区（公共池）
-- - claimed：私有区（已绑定领取人）
-- - quarantine：私有区→温养区（局部损坏，7 天后回公有区）
-- - repair：维修区（客户端声称 full damage 后进入；等待维修者处理）
-- - repair_claimed：维修区（已被维修者领取处理中）
-- - deleted：历史兼容字段（旧逻辑：完全损坏直接删除；新逻辑不再直接进入 deleted）
CREATE TABLE IF NOT EXISTS artworks (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  artwork_id TEXT NOT NULL UNIQUE,
  upload_key_hash TEXT NOT NULL,

  status TEXT NOT NULL DEFAULT 'available',

  -- claim 绑定（私有池）
  claimed_by_user_key_hash TEXT,
  claimed_by_upload_key_hash TEXT,
  claimed_at TEXT,

  -- 局部损坏温养
  eligible_after TEXT,

  -- 维修区：维修尝试
  repair_fail_count INTEGER NOT NULL DEFAULT 0,
  repair_claimed_by_upload_key_hash TEXT,
  repair_claimed_at TEXT,
  repair_last_fail_note TEXT,
  repair_last_failed_at TEXT,

  -- 旧字段（兼容保留）
  deleted_reason TEXT,
  deleted_at TEXT,

  created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_artworks_status ON artworks(status);
CREATE INDEX IF NOT EXISTS idx_artworks_claimed_by_user ON artworks(claimed_by_user_key_hash);
CREATE INDEX IF NOT EXISTS idx_artworks_claimed_by_upload ON artworks(claimed_by_upload_key_hash);
CREATE INDEX IF NOT EXISTS idx_artworks_eligible_after ON artworks(eligible_after);
CREATE INDEX IF NOT EXISTS idx_artworks_repair_fail_count ON artworks(repair_fail_count);

-- artwork_objects：R2 对象键（去敏后的作品 JSON）
CREATE TABLE IF NOT EXISTS artwork_objects (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  artwork_id TEXT NOT NULL,
  r2_key TEXT NOT NULL,
  size_bytes INTEGER NOT NULL,
  created_at TEXT NOT NULL,
  UNIQUE(artwork_id)
);
CREATE INDEX IF NOT EXISTS idx_artwork_objects_artwork_id ON artwork_objects(artwork_id);

-- artwork_damage_reports：损坏上报（完全/局部），先信任客户端并写入待校验队列
CREATE TABLE IF NOT EXISTS artwork_damage_reports (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  artwork_id TEXT NOT NULL,
  reporter_key_hash TEXT NOT NULL,
  reporter_role TEXT NOT NULL, -- user | upload
  kind TEXT NOT NULL,          -- full | partial
  reported_at TEXT NOT NULL,
  note TEXT
);
CREATE INDEX IF NOT EXISTS idx_artwork_damage_reports_artwork_id ON artwork_damage_reports(artwork_id);
CREATE INDEX IF NOT EXISTS idx_artwork_damage_reports_reported_at ON artwork_damage_reports(reported_at);

-- pending_verification_artworks：待二次校验队列（服务端暂时无法校验时）
CREATE TABLE IF NOT EXISTS pending_verification_artworks (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  artwork_id TEXT NOT NULL UNIQUE,
  last_report_kind TEXT,
  last_reported_at TEXT NOT NULL
);

-- artwork_tombstones：墓地区“token 标记”，用于防止永久失败的作品被重复提交
-- 说明：墓地区的正文可在 R2 侧保留（例如 graveyard/artworks/<artwork_id>.json），数据库侧仅保留 tombstone。
CREATE TABLE IF NOT EXISTS artwork_tombstones (
  artwork_id TEXT PRIMARY KEY,
  reason TEXT,
  note TEXT,
  created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_artwork_tombstones_created_at ON artwork_tombstones(created_at);

-- uploader_stats：热心群众激励数据（有效投稿数、分发资格）
CREATE TABLE IF NOT EXISTS uploader_stats (
  upload_key_hash TEXT PRIMARY KEY,
  valid_artworks_submitted INTEGER NOT NULL DEFAULT 0,
  distribution_credits INTEGER NOT NULL DEFAULT 0,
  updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_uploader_stats_valid ON uploader_stats(valid_artworks_submitted);
CREATE INDEX IF NOT EXISTS idx_uploader_stats_credits ON uploader_stats(distribution_credits);

-- package_batch_issuers：记录“分发包批次”的发起者（admin / upload）
CREATE TABLE IF NOT EXISTS package_batch_issuers (
  batch_id TEXT PRIMARY KEY,
  issuer_key_hash TEXT NOT NULL,
  issuer_role TEXT NOT NULL,
  created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_package_batch_issuers_issuer ON package_batch_issuers(issuer_key_hash);

-- client_activity：记录“某个 key 最近一次发起请求的时间”，用于实现
-- “如果用户 7 天都没有对平台发起任何请求，则视为离开；其已领取作品解绑回公共池”。
CREATE TABLE IF NOT EXISTS client_activity (
  key_hash TEXT PRIMARY KEY,
  role TEXT NOT NULL, -- user | upload
  last_seen_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_client_activity_last_seen_at ON client_activity(last_seen_at);
