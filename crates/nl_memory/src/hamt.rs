//! HAMT - 分层抽象记忆树
//!
//! 实现 20 字标签 -> 200 字摘要 -> 全量提取的漏斗检索。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 记忆层级
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryLevel {
    /// Level 1: 20 字特征标签 (RAM 热区)
    Tag = 1,
    /// Level 2: 200 字逻辑摘要 (RAM 热区)
    Summary = 2,
    /// Level 3: 全量原始数据 (Disk 冷区)
    Full = 3,
}

/// 记忆条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// 记忆 ID
    pub id: Uuid,
    /// 20 字特征标签
    pub tag: String,
    /// 200 字逻辑摘要
    pub summary: String,
    /// 全量数据路径 (磁盘)
    pub full_data_path: Option<String>,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 最后访问时间
    pub last_accessed: chrono::DateTime<chrono::Utc>,
    /// 访问次数
    pub access_count: u64,
    /// 元数据
    pub metadata: HashMap<String, String>,
}

impl MemoryEntry {
    /// 创建新记忆条目
    pub fn new(tag: impl Into<String>, summary: impl Into<String>) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4(),
            tag: tag.into(),
            summary: summary.into(),
            full_data_path: None,
            created_at: now,
            last_accessed: now,
            access_count: 0,
            metadata: HashMap::new(),
        }
    }

    /// 记录访问
    pub fn touch(&mut self) {
        self.last_accessed = chrono::Utc::now();
        self.access_count += 1;
    }
}

/// HAMT 索引
pub struct HamtIndex {
    /// 标签索引 (Level 1)
    tag_index: HashMap<String, Uuid>,
    /// 摘要缓存 (Level 2)
    summary_cache: HashMap<Uuid, String>,
    /// 所有条目
    entries: HashMap<Uuid, MemoryEntry>,
}

impl HamtIndex {
    /// 创建新的 HAMT 索引
    pub fn new() -> Self {
        Self {
            tag_index: HashMap::new(),
            summary_cache: HashMap::new(),
            entries: HashMap::new(),
        }
    }

    /// 存储记忆
    pub fn store(&mut self, entry: MemoryEntry) {
        let id = entry.id;
        self.tag_index.insert(entry.tag.clone(), id);
        self.summary_cache.insert(id, entry.summary.clone());
        self.entries.insert(id, entry);
    }

    /// 通过标签检索 (Level 1 -> Level 2 -> Level 3)
    pub fn retrieve_by_tag(&mut self, tag: &str) -> Option<&MemoryEntry> {
        if let Some(id) = self.tag_index.get(tag) {
            if let Some(entry) = self.entries.get_mut(id) {
                entry.touch();
                return Some(entry);
            }
        }
        None
    }

    /// 模糊搜索标签
    pub fn search_tags(&self, query: &str) -> Vec<&MemoryEntry> {
        self.entries
            .values()
            .filter(|e| e.tag.contains(query))
            .collect()
    }

    /// 搜索摘要
    pub fn search_summaries(&self, query: &str) -> Vec<&MemoryEntry> {
        self.entries
            .values()
            .filter(|e| e.summary.contains(query))
            .collect()
    }

    /// 获取所有条目
    pub fn all_entries(&self) -> Vec<&MemoryEntry> {
        self.entries.values().collect()
    }

    /// 获取条目数量
    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// 清理冷数据
    pub fn prune_cold(&mut self, days: i64) -> usize {
        let threshold = chrono::Utc::now() - chrono::Duration::days(days);
        let cold_ids: Vec<Uuid> = self
            .entries
            .iter()
            .filter(|(_, e)| e.last_accessed < threshold)
            .map(|(id, _)| *id)
            .collect();

        let removed = cold_ids.len();
        for id in &cold_ids {
            if let Some(entry) = self.entries.remove(id) {
                self.tag_index.remove(&entry.tag);
                self.summary_cache.remove(id);
            }
        }
        removed
    }
}

impl Default for HamtIndex {
    fn default() -> Self {
        Self::new()
    }
}
