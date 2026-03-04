-- 0010_user_keys_v2_backfill.sql
-- 目标：将 v1 user_keys 历史数据补齐到 v2（users_v2 / user_keys_v2），作为 v2 单一真源迁移。
-- 说明：
-- - 幂等：可重复执行，不会重复写入。
-- - user_id 规则：u_<key_hash>
-- - role 固定为 user
-- - enabled 继承自 v1.user_keys.enabled

-- 1) 先为 v1 key 生成对应 users_v2（若不存在）
INSERT OR IGNORE INTO users_v2(
  id,
  display_name,
  roles,
  current_account_ids,
  daily_refill_limit,
  account_limit_delta,
  disabled,
  created_at,
  updated_at
)
SELECT
  'u_' || uk.key_hash AS id,
  CASE
    WHEN uk.label IS NULL OR trim(uk.label) = '' THEN 'migrated:' || substr(uk.key_hash, 1, 8)
    ELSE uk.label
  END AS display_name,
  'user' AS roles,
  '[]' AS current_account_ids,
  200 AS daily_refill_limit,
  0 AS account_limit_delta,
  CASE WHEN coalesce(uk.enabled, 1) = 1 THEN 0 ELSE 1 END AS disabled,
  coalesce(uk.created_at, strftime('%Y-%m-%dT%H:%M:%fZ','now')) AS created_at,
  strftime('%Y-%m-%dT%H:%M:%fZ','now') AS updated_at
FROM user_keys uk;

-- 2) 补齐 user_keys_v2（若不存在）
INSERT OR IGNORE INTO user_keys_v2(
  key_hash,
  user_id,
  role,
  enabled,
  created_at,
  updated_at
)
SELECT
  uk.key_hash,
  'u_' || uk.key_hash AS user_id,
  'user' AS role,
  CASE WHEN coalesce(uk.enabled, 1) = 1 THEN 1 ELSE 0 END AS enabled,
  coalesce(uk.created_at, strftime('%Y-%m-%dT%H:%M:%fZ','now')) AS created_at,
  strftime('%Y-%m-%dT%H:%M:%fZ','now') AS updated_at
FROM user_keys uk;

-- 3) 对已存在的 user_keys_v2 进行一次 enabled 同步（v1 -> v2）
UPDATE user_keys_v2
SET enabled = (
      SELECT CASE WHEN coalesce(uk.enabled, 1) = 1 THEN 1 ELSE 0 END
      FROM user_keys uk
      WHERE uk.key_hash = user_keys_v2.key_hash
    ),
    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
WHERE role = 'user'
  AND key_hash IN (SELECT key_hash FROM user_keys);

-- 4) 回写 users_v2.disabled：只要某 user_id 存在 enabled=1 的 user_keys_v2，则 disabled=0，否则=1
UPDATE users_v2
SET disabled = CASE
      WHEN EXISTS (
        SELECT 1 FROM user_keys_v2 k2
        WHERE k2.user_id = users_v2.id
          AND k2.role = 'user'
          AND coalesce(k2.enabled, 1) = 1
      ) THEN 0 ELSE 1 END,
    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
WHERE id IN (
  SELECT DISTINCT 'u_' || key_hash FROM user_keys
);