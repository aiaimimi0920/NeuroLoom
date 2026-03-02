-- 仅保留指定 20 组用户（按 key_hash），其余用户/绑定全部清理

CREATE TEMP TABLE keep_hashes (key_hash TEXT PRIMARY KEY);
INSERT INTO keep_hashes(key_hash) VALUES
('bee382f3b07edeb8529cb142201a2a4542fc914da30baa7914f3cbbeb64bae71'),
('88218e6382d4d1662e30a38d7711abf65ddd4c1720ea8d9846f54f06da6412d9'),
('83590d315d81e166da71252dea85876034253c3456233c7fda15e0cad52df5f8'),
('0d7a57663d717d0ffc87ffcb88c9f2b30f1c4c16dbcf9e97f763c562d675e71d'),
('3d78be1b1e576b23e2644225c9c9af72112ed738c32f953be9e6bb50db982b57'),
('af39a9191112a97ded1af5fa278c3c0e3a62b8756b1920a693f2d73ac9f5906a'),
('fd418a6425a909aa65ad17e8b6f1154448a5d328cc998f8a269013dea7c7f538'),
('6503bfdfe6a1422c7268ce5921490f342f9798d78d2c4563ec05da67198cd8b4'),
('e0019ed371ee2b12eb1b98631576d475e57dc7afeaade3eb9595e9859deb66e1'),
('23fba8b507a18377ed3770c82541498470a8722be5f1a5b4f2987fd5f9ea11f5'),
('929a410749c1dff425885da430485c0d6d3f0f34903b53412a2812ef34bc8931'),
('d76d95324f8c9dfcc7de6e32f83e5781efd5126e9fa5792744183fe8dc61ff6e'),
('7724a33686a331f903ca1fb124f8cfa73d73e9a3f49de113041a63ef65ac64c4'),
('c4bc36b420001dc58948c725928f0f6017d059b49dda14a66d6a824f24375f96'),
('94e93c9b9d17fcf4340cd82e66640f95b9f005aa794be84e27e1ca9783e99d92'),
('04cd127876d7b9567b2c248acafaae44806d60da2fabbbff5698d441b13effd2'),
('3595425c8864ac7be240ff90841de601220b8ad4349e924ab5cf1c7a871b72ed'),
('5cd774c4b58c9fe557d55767af195eeef04aa5477a6ec64d06964fcbd06b7c21'),
('d24b98a348c6f89abdae46aa690dc07469bf10b294e6ef23a9a378d6add3bd2d'),
('1cb50055bccd187413f1ecdc9d51a0490456a989e5392391a7ee0ab70683908a');

-- 释放非保留用户的私有账号回公有池
UPDATE accounts_v2
SET owner='-1', updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now')
WHERE owner NOT IN ('-1','-2','-3','-4')
  AND owner NOT IN (SELECT key_hash FROM keep_hashes);

-- 清理非保留用户在二级绑定体系里的关系与用户实体
DELETE FROM user_daily_refill_usage
WHERE user_id IN (
  SELECT id FROM users_v2
  WHERE id NOT IN (
    SELECT DISTINCT user_id FROM user_keys_v2
    WHERE key_hash IN (SELECT key_hash FROM keep_hashes)
  )
);

DELETE FROM user_daily_sync_all_usage
WHERE user_id IN (
  SELECT id FROM users_v2
  WHERE id NOT IN (
    SELECT DISTINCT user_id FROM user_keys_v2
    WHERE key_hash IN (SELECT key_hash FROM keep_hashes)
  )
);

DELETE FROM user_keys_v2
WHERE key_hash NOT IN (SELECT key_hash FROM keep_hashes);

DELETE FROM users_v2
WHERE id NOT IN (
  SELECT DISTINCT user_id FROM user_keys_v2
);

-- 清理老 user_keys 中非保留 key（不影响保留 20 个）
DELETE FROM user_keys
WHERE key_hash NOT IN (SELECT key_hash FROM keep_hashes);

-- 释放非保留用户占用的 refill key / artworks 绑定
UPDATE refill_keys
SET status='available', claimed_by_upload_key_hash=NULL, claimed_by_user_key_hash=NULL, claimed_at=NULL
WHERE claimed_by_user_key_hash IS NOT NULL
  AND claimed_by_user_key_hash NOT IN (SELECT key_hash FROM keep_hashes);

UPDATE artworks
SET status='available', claimed_by_user_key_hash=NULL, claimed_by_upload_key_hash=NULL, claimed_at=NULL, eligible_after=NULL
WHERE claimed_by_user_key_hash IS NOT NULL
  AND claimed_by_user_key_hash NOT IN (SELECT key_hash FROM keep_hashes);

DROP TABLE keep_hashes;
