//! MoA 议会 - 多模型混合专家议会

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 议会成员
#[derive(Debug, Clone)]
pub struct ParliamentMember {
    /// 成员 ID
    pub id: Uuid,
    /// 模型名称
    pub model: String,
    /// 权重
    pub weight: f64,
}

impl ParliamentMember {
    pub fn new(model: impl Into<String>, weight: f64) -> Self {
        Self {
            id: Uuid::new_v4(),
            model: model.into(),
            weight,
        }
    }
}

/// MoA 议会
pub struct Parliament {
    /// 成员列表
    members: Vec<ParliamentMember>,
    /// 主席模型 (负责归纳)
    pub chair_model: String,
}

impl Parliament {
    /// 创建新议会
    pub fn new() -> Self {
        Self {
            members: vec![
                ParliamentMember::new("claude-3-5-sonnet", 1.0),
                ParliamentMember::new("gpt-4", 1.0),
                ParliamentMember::new("gemini-pro", 0.8),
            ],
            chair_model: "claude-3-5-sonnet".to_string(),
        }
    }

    /// 添加成员
    pub fn add_member(&mut self, member: ParliamentMember) {
        self.members.push(member);
    }

    /// 召开议会
    pub async fn convene(&self, question: &str) -> nl_core::Result<ParliamentDecision> {
        // TODO: 实现实际的议会决策逻辑
        Ok(ParliamentDecision {
            question: question.to_string(),
            consensus: "Default consensus".to_string(),
            votes: self.members.iter().map(|m| m.model.clone()).collect(),
            confidence: 0.8,
        })
    }

    /// 获取成员数量
    pub fn member_count(&self) -> usize {
        self.members.len()
    }
}

impl Default for Parliament {
    fn default() -> Self {
        Self::new()
    }
}

/// 议会决策
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParliamentDecision {
    /// 问题
    pub question: String,
    /// 共识结论
    pub consensus: String,
    /// 投票成员
    pub votes: Vec<String>,
    /// 置信度
    pub confidence: f64,
}
