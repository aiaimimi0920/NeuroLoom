-- 0002_repairer.sql
-- 引入“维修者/维修区/墓地区”相关字段与 tombstone 表。
-- 注意：D1(SQLite) 的 ALTER TABLE ADD COLUMN 不支持 IF NOT EXISTS。
-- 本迁移应只执行一次；重复执行会报 duplicate column。

-- 1) artworks：新增维修相关列
ALTER TABLE artworks ADD COLUMN repair_fail_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE artworks ADD COLUMN repair_claimed_by_upload_key_hash TEXT;
ALTER TABLE artworks ADD COLUMN repair_claimed_at TEXT;
ALTER TABLE artworks ADD COLUMN repair_last_fail_note TEXT;
ALTER TABLE artworks ADD COLUMN repair_last_failed_at TEXT;

CREATE INDEX IF NOT EXISTS idx_artworks_repair_fail_count ON artworks(repair_fail_count);

-- 2) 墓地区：用于“永久失败的作品”只保留 token 标记，防止后续重复提交。
CREATE TABLE IF NOT EXISTS artwork_tombstones (
  artwork_id TEXT PRIMARY KEY,
  reason TEXT,
  note TEXT,
  created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_artwork_tombstones_created_at ON artwork_tombstones(created_at);
