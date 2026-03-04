-- 0009_refill_owner_compat_rollback.sql
-- 回滚仅恢复 owner 语义最小兼容：
-- 新语义中的 -4(休眠) / -5(待置信) 都收敛回旧语义可识别状态。
--
-- 说明：
-- - 回滚不删除表结构，避免误删已产生的审计数据。
-- - 仅调整 owner，确保旧代码不会因为未知状态而异常。

-- 待置信区回到公有池
UPDATE accounts_v2
SET owner='-1', updated_at=strftime('%Y-%m-%dT%H:%M:%SZ','now')
WHERE owner='-5';

-- 休眠区回到“维修中”（旧语义）
UPDATE accounts_v2
SET owner='-4', updated_at=strftime('%Y-%m-%dT%H:%M:%SZ','now')
WHERE owner='-4';

-- 清理休眠索引数据（可选，不删表）
DELETE FROM account_v2_sleep;
