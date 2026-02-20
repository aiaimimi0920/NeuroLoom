//! 事件存储引擎

use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use nl_core::event::{Event, EventKind};
use nl_core::entity::EntityId;
use nl_core::Result;

/// 事件存储配置
#[derive(Debug, Clone)]
pub struct EventStoreConfig {
    /// 数据库路径
    pub database_path: String,
    /// 批量写入大小
    pub batch_size: usize,
    /// 是否启用 WAL
    pub enable_wal: bool,
}

impl Default for EventStoreConfig {
    fn default() -> Self {
        Self {
            database_path: "neuroloom.db".to_string(),
            batch_size: 100,
            enable_wal: true,
        }
    }
}

/// 事件存储
pub struct EventStore {
    config: EventStoreConfig,
    /// 事件缓冲区
    buffer: Vec<Event>,
}

impl EventStore {
    /// 创建新的事件存储
    pub fn new(config: EventStoreConfig) -> Self {
        Self {
            config,
            buffer: Vec::new(),
        }
    }

    /// 从路径创建
    pub async fn open(path: impl AsRef<Path>) -> Result<Self> {
        let config = EventStoreConfig {
            database_path: path.as_ref().to_string_lossy().to_string(),
            ..Default::default()
        };
        Ok(Self::new(config))
    }

    /// 追加事件
    pub async fn append(&mut self, event: Event) -> Result<()> {
        self.buffer.push(event);

        if self.buffer.len() >= self.config.batch_size {
            self.flush().await?;
        }

        Ok(())
    }

    /// 批量追加
    pub async fn append_batch(&mut self, events: Vec<Event>) -> Result<()> {
        self.buffer.extend(events);
        self.flush().await
    }

    /// 刷新缓冲区到持久化存储
    pub async fn flush(&mut self) -> Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        // TODO: 实现实际的 SQLite 写入
        // 这里是骨架实现
        self.buffer.clear();
        Ok(())
    }

    /// 查询实体的事件流
    pub async fn get_events(&self, entity_id: EntityId) -> Result<Vec<Event>> {
        // TODO: 实现 SQLite 查询
        Ok(vec![])
    }

    /// 查询时间范围内的事件
    pub async fn get_events_by_time(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Event>> {
        // TODO: 实现 SQLite 查询
        Ok(vec![])
    }

    /// 获取事件计数
    pub async fn count(&self) -> Result<u64> {
        Ok(0)
    }
}

/// 事件流迭代器
pub struct EventStream {
    events: Vec<Event>,
    position: usize,
}

impl EventStream {
    pub fn new(events: Vec<Event>) -> Self {
        Self { events, position: 0 }
    }

    pub fn next(&mut self) -> Option<&Event> {
        if self.position < self.events.len() {
            let event = &self.events[self.position];
            self.position += 1;
            Some(event)
        } else {
            None
        }
    }

    pub fn position(&self) -> usize {
        self.position
    }
}
