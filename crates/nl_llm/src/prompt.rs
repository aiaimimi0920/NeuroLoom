//! Prompt 构建器与基数树缓存
//!
//! 实现动静分离的前缀缓存优化。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Prompt 片段类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PromptSegment {
    /// 静态片段 (可缓存)
    Static(String),
    /// 动态片段 (实时生成)
    Dynamic(String),
}

/// Prompt 构建器
#[derive(Debug, Default)]
pub struct PromptBuilder {
    /// 片段列表
    segments: Vec<PromptSegment>,
    /// 元数据
    metadata: HashMap<String, String>,
}

impl PromptBuilder {
    /// 创建新的构建器
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加静态片段 (系统设定、SOP 规则等)
    pub fn add_static(mut self, content: impl Into<String>) -> Self {
        self.segments.push(PromptSegment::Static(content.into()));
        self
    }

    /// 添加动态片段 (用户输入、实时数据)
    pub fn add_dynamic(mut self, content: impl Into<String>) -> Self {
        self.segments.push(PromptSegment::Dynamic(content.into()));
        self
    }

    /// 添加元数据
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// 构建最终 Prompt
    pub fn build(&self) -> String {
        self.segments
            .iter()
            .map(|s| match s {
                PromptSegment::Static(s) => s.clone(),
                PromptSegment::Dynamic(s) => s.clone(),
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// 计算静态前缀哈希 (用于缓存键)
    pub fn static_prefix_hash(&self) -> String {
        let static_content: String = self
            .segments
            .iter()
            .filter_map(|s| match s {
                PromptSegment::Static(s) => Some(s.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");

        let mut hasher = Sha256::new();
        hasher.update(static_content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// 获取静态内容长度 (用于预估缓存效果)
    pub fn static_content_len(&self) -> usize {
        self.segments
            .iter()
            .filter_map(|s| match s {
                PromptSegment::Static(s) => Some(s.len()),
                _ => None,
            })
            .sum()
    }

    /// 获取动态内容长度
    pub fn dynamic_content_len(&self) -> usize {
        self.segments
            .iter()
            .filter_map(|s| match s {
                PromptSegment::Dynamic(s) => Some(s.len()),
                _ => None,
            })
            .sum()
    }

    /// 分离静态和动态内容
    pub fn split(&self) -> (String, String) {
        let static_part: String = self
            .segments
            .iter()
            .filter_map(|s| match s {
                PromptSegment::Static(s) => Some(s.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");

        let dynamic_part: String = self
            .segments
            .iter()
            .filter_map(|s| match s {
                PromptSegment::Dynamic(s) => Some(s.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");

        (static_part, dynamic_part)
    }
}

/// 基数树节点
#[derive(Debug, Default)]
struct RadixNode {
    /// 子节点
    children: HashMap<char, RadixNode>,
    /// 是否为完整键
    is_key: bool,
    /// 缓存值
    value: Option<String>,
}

/// 基数树缓存 (用于前缀匹配)
#[derive(Debug, Default)]
pub struct RadixTreeCache {
    root: RadixNode,
}

impl RadixTreeCache {
    /// 创建新的基数树缓存
    pub fn new() -> Self {
        Self::default()
    }

    /// 插入缓存
    pub fn insert(&mut self, key: &str, value: String) {
        let mut node = &mut self.root;
        for ch in key.chars() {
            node = node.children.entry(ch).or_default();
        }
        node.is_key = true;
        node.value = Some(value);
    }

    /// 查找最长前缀匹配
    pub fn find_longest_prefix<'a>(&self, key: &'a str) -> Option<&'a str> {
        let mut node = &self.root;
        let mut last_match: Option<&str> = None;

        for (i, ch) in key.chars().enumerate() {
            if let Some(child) = node.children.get(&ch) {
                node = child;
                if node.is_key {
                    last_match = Some(&key[..=i]);
                }
            } else {
                break;
            }
        }

        last_match
    }

    /// 获取缓存值
    pub fn get(&self, key: &str) -> Option<&String> {
        let mut node = &self.root;
        for ch in key.chars() {
            node = node.children.get(&ch)?;
        }
        if node.is_key {
            node.value.as_ref()
        } else {
            None
        }
    }

    /// 检查键是否存在
    pub fn contains(&self, key: &str) -> bool {
        self.get(key).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_builder() {
        let prompt = PromptBuilder::new()
            .add_static("You are a helpful assistant.")
            .add_dynamic("User asks: Hello!")
            .build();

        assert!(prompt.contains("helpful assistant"));
        assert!(prompt.contains("Hello"));
    }

    #[test]
    fn test_radix_tree() {
        let mut cache = RadixTreeCache::new();
        cache.insert("system_prompt", "cached_content".to_string());

        assert!(cache.contains("system_prompt"));
        assert_eq!(cache.get("system_prompt"), Some(&"cached_content".to_string()));
    }
}
