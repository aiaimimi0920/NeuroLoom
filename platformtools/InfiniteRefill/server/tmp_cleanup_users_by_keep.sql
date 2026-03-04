BEGIN TRANSACTION;

-- 1) 解除被删除用户与账户绑定（回收到公共池）
UPDATE accounts_v2
SET owner='-1',
    updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now')
WHERE owner IN (
  SELECT key_hash
  FROM user_keys_v2
  WHERE role='user'
    AND key_hash NOT IN (SELECT key_hash FROM keep_hashes_tmp)
);

-- 2) 清理按用户 key 绑定的 v1/v2 关联字段
UPDATE refill_keys
SET claimed_by_user_key_hash=NULL,
    claimed_at=NULL,
    status='available'
WHERE claimed_by_user_key_hash IN (
  SELECT key_hash
  FROM user_keys_v2
  WHERE role='user'
    AND key_hash NOT IN (SELECT key_hash FROM keep_hashes_tmp)
);

UPDATE artworks
SET claimed_by_user_key_hash=NULL,
    claimed_at=NULL,
    status='available'
WHERE claimed_by_user_key_hash IN (
  SELECT key_hash
  FROM user_keys_v2
  WHERE role='user'
    AND key_hash NOT IN (SELECT key_hash FROM keep_hashes_tmp)
);

-- 3) 删除与被删 key / user 相关的行为与审计记录
DELETE FROM topup_issues
WHERE issued_to_key_hash IN (
  SELECT key_hash
  FROM user_keys_v2
  WHERE role='user'
    AND key_hash NOT IN (SELECT key_hash FROM keep_hashes_tmp)
);

DELETE FROM sync_all_risk_events
WHERE key_hash IN (
  SELECT key_hash
  FROM user_keys_v2
  WHERE role='user'
    AND key_hash NOT IN (SELECT key_hash FROM keep_hashes_tmp)
);

DELETE FROM account_confidence_queue
WHERE first_reporter_key_hash IN (
    SELECT key_hash FROM user_keys_v2 WHERE role='user' AND key_hash NOT IN (SELECT key_hash FROM keep_hashes_tmp)
  )
   OR last_replayed_to_key_hash IN (
    SELECT key_hash FROM user_keys_v2 WHERE role='user' AND key_hash NOT IN (SELECT key_hash FROM keep_hashes_tmp)
  )
   OR last_feedback_key_hash IN (
    SELECT key_hash FROM user_keys_v2 WHERE role='user' AND key_hash NOT IN (SELECT key_hash FROM keep_hashes_tmp)
  );

DELETE FROM account_confidence_replays
WHERE replayed_to_key_hash IN (
    SELECT key_hash FROM user_keys_v2 WHERE role='user' AND key_hash NOT IN (SELECT key_hash FROM keep_hashes_tmp)
  )
   OR source_reporter_key_hash IN (
    SELECT key_hash FROM user_keys_v2 WHERE role='user' AND key_hash NOT IN (SELECT key_hash FROM keep_hashes_tmp)
  );

DELETE FROM user_daily_refill_usage
WHERE user_id IN (
  SELECT DISTINCT user_id
  FROM user_keys_v2
  WHERE role='user'
    AND key_hash NOT IN (SELECT key_hash FROM keep_hashes_tmp)
);

DELETE FROM user_daily_sync_all_usage
WHERE user_id IN (
  SELECT DISTINCT user_id
  FROM user_keys_v2
  WHERE role='user'
    AND key_hash NOT IN (SELECT key_hash FROM keep_hashes_tmp)
);

DELETE FROM user_daily_untrust_events
WHERE user_id IN (
  SELECT DISTINCT user_id
  FROM user_keys_v2
  WHERE role='user'
    AND key_hash NOT IN (SELECT key_hash FROM keep_hashes_tmp)
);

DELETE FROM user_ban_audit
WHERE user_id IN (
  SELECT DISTINCT user_id
  FROM user_keys_v2
  WHERE role='user'
    AND key_hash NOT IN (SELECT key_hash FROM keep_hashes_tmp)
);

-- 4) 删除用户主数据（v2）
DELETE FROM user_keys_v2
WHERE role='user'
  AND key_hash NOT IN (SELECT key_hash FROM keep_hashes_tmp);

DELETE FROM users_v2
WHERE id NOT IN (SELECT DISTINCT user_id FROM user_keys_v2);

-- 5) 兼容层 v1：仅保留名单中的用户 key
DELETE FROM user_keys
WHERE key_hash NOT IN (SELECT key_hash FROM keep_hashes_tmp);

COMMIT;
