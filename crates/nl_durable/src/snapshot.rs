//! 快照管理器

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use nl_core::entity::EntityId;
use nl_core::Result;

/// 快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// 快照 ID
    pub id: Uuid,
    /// 实体 ID
    pub entity_id: EntityId,
    /// 快照时间
    pub timestamp: DateTime<Utc>,
    /// 事件版本 (快照点)
    pub event_version: u64,
    /// 状态数据
    pub state: serde_json::Value,
}

impl Snapshot {
    /// 创建新快照
    pub fn new(entity_id: EntityId, event_version: u64, state: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            entity_id,
            timestamp: Utc::now(),
            event_version,
            state,
        }
    }
}

/// 快照策略
#[derive(Debug, Clone)]
pub enum SnapshotStrategy {
    /// 每 N 个事件创建快照
    EveryNEvents(u64),
    /// 时间间隔
    TimeInterval(chrono::Duration),
    /// 自定义条件
    Custom(Box<dyn Fn(u64) -> bool + Send + Sync>),
}

/// 快照管理器
pub struct SnapshotManager {
    /// 快照策略
    strategy: SnapshotStrategy,
    /// 内存缓存
    cache: Vec<Snapshot>,
    /// 上次快照版本
    last_snapshot_version: u64,
}

impl SnapshotManager {
    /// 创建新的快照管理器
    pub fn new(strategy: SnapshotStrategy) -> Self {
        Self {
            strategy,
            cache: Vec::new(),
            last_snapshot_version: 0,
        }
    }

    /// 创建默认策略的管理器
    pub fn default_manager() -> Self {
        Self::new(SnapshotStrategy::EveryNEvents(100))
    }

    /// 检查是否需要创建快照
    pub fn should_snapshot(&self, current_version: u64) -> bool {
        match &self.strategy {
            SnapshotStrategy::EveryNEvents(n) => {
                current_version - self.last_snapshot_version >= *n
            }
            SnapshotStrategy::TimeInterval(_) => {
                // TODO: 实现时间间隔检查
                false
            }
            SnapshotStrategy::Custom(predicate) => predicate(current_version),
        }
    }

    /// 创建快照
    pub async fn create_snapshot(
        &mut self,
        entity_id: EntityId,
        event_version: u64,
        state: serde_json::Value,
    ) -> Result<Snapshot> {
        let snapshot = Snapshot::new(entity_id, event_version, state);
        self.cache.push(snapshot.clone());
        self.last_snapshot_version = event_version;
        Ok(snapshot)
    }

    /// 获取实体的最新快照
    pub fn get_latest_snapshot(&self, entity_id: EntityId) -> Option<&Snapshot> {
        self.cache
            .iter()
            .filter(|s| s.entity_id == entity_id)
            .max_by_key(|s| s.event_version)
    }

    /// 获取指定版本的快照
    pub fn get_snapshot_at_version(&self, entity_id: EntityId, version: u64) -> Option<&Snapshot> {
        self.cache
            .iter()
            .filter(|s| s.entity_id == entity_id && s.event_version <= version)
            .max_by_key(|s| s.event_version)
    }

    /// 清理旧快照
    pub async fn prune_old_snapshots(&mut self, keep_last: usize) -> Result<()> {
        // TODO: 实现基于时间的清理
        if self.cache.len() > keep_last {
            self.cache.drain(0..self.cache.len() - keep_last);
        }
        Ok(())
    }

    /// 获取快照数量
    pub fn count(&self) -> usize {
        self.cache.len()
    }
}
