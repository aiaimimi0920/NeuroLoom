//! 语义帧差分感知器

use serde::{Deserialize, Serialize};

/// 帧差分结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameDiff {
    /// 是否有显著变化
    pub significant_change: bool,
    /// 变化区域
    pub changed_regions: Vec<Region>,
    /// 相似度分数
    pub similarity: f64,
}

/// 变化区域
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Region {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub change_type: ChangeType,
}

/// 变化类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChangeType {
    /// 新增内容
    Added,
    /// 删除内容
    Removed,
    /// 修改内容
    Modified,
}

/// 语义差分器
pub struct SemanticDiff {
    /// 差分阈值
    threshold: f64,
    /// 上一帧
    prev_frame: Option<Vec<u8>>,
}

impl SemanticDiff {
    /// 创建新差分器
    pub fn new(threshold: f64) -> Self {
        Self {
            threshold,
            prev_frame: None,
        }
    }

    /// 创建默认差分器
    pub fn default_diff() -> Self {
        Self::new(0.1)
    }

    /// 处理新帧
    pub fn process(&mut self, frame: &[u8]) -> FrameDiff {
        if let Some(prev) = &self.prev_frame {
            let similarity = self.calculate_similarity(prev, frame);
            let significant = similarity < self.threshold;

            self.prev_frame = Some(frame.to_vec());

            FrameDiff {
                significant_change: significant,
                changed_regions: Vec::new(),
                similarity,
            }
        } else {
            self.prev_frame = Some(frame.to_vec());
            FrameDiff {
                significant_change: true,
                changed_regions: Vec::new(),
                similarity: 0.0,
            }
        }
    }

    /// 计算相似度
    fn calculate_similarity(&self, a: &[u8], b: &[u8]) -> f64 {
        if a.len() != b.len() {
            return 0.0;
        }

        let same = a.iter().zip(b.iter()).filter(|(x, y)| x == y).count();
        same as f64 / a.len() as f64
    }

    /// 重置
    pub fn reset(&mut self) {
        self.prev_frame = None;
    }
}

impl Default for SemanticDiff {
    fn default() -> Self {
        Self::default_diff()
    }
}
