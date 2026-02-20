//! Critic Agent

use uuid::Uuid;

/// Critic Agent - 审查和质疑
pub struct Critic {
    /// Critic ID
    pub id: Uuid,
    /// 严格程度
    pub strictness: f64,
}

impl Critic {
    /// 创建新 Critic
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            strictness: 0.7,
        }
    }

    /// 审查结果
    pub async fn review(&self, work: &str) -> nl_core::Result<ReviewResult> {
        // TODO: 实现实际的审查逻辑
        Ok(ReviewResult {
            approved: true,
            issues: Vec::new(),
            suggestions: Vec::new(),
        })
    }
}

impl Default for Critic {
    fn default() -> Self {
        Self::new()
    }
}

/// 审查结果
#[derive(Debug, Clone)]
pub struct ReviewResult {
    /// 是否通过
    pub approved: bool,
    /// 发现的问题
    pub issues: Vec<String>,
    /// 改进建议
    pub suggestions: Vec<String>,
}
