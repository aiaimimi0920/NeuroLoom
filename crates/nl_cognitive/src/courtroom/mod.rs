//! 法庭模块 - Worker, Critic, MoA 议会

pub mod worker;
pub mod critic;
pub mod parliament;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 裁决结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verdict {
    /// 裁决 ID
    pub id: Uuid,
    /// 任务 ID
    pub task_id: Uuid,
    /// 是否通过
    pub passed: bool,
    /// 评分
    pub score: f64,
    /// 理由
    pub reasoning: String,
    /// 修改建议
    pub suggestions: Vec<String>,
}

impl Verdict {
    /// 创建通过的裁决
    pub fn approved(task_id: Uuid, score: f64, reasoning: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            task_id,
            passed: true,
            score,
            reasoning: reasoning.into(),
            suggestions: Vec::new(),
        }
    }

    /// 创建拒绝的裁决
    pub fn rejected(task_id: Uuid, score: f64, reasoning: impl Into<String>, suggestions: Vec<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            task_id,
            passed: false,
            score,
            reasoning: reasoning.into(),
            suggestions,
        }
    }
}

/// 法庭 - 协调 Worker 和 Critic
pub struct Courtroom {
    /// 最大审议轮数
    max_rounds: u32,
}

impl Courtroom {
    /// 创建新法庭
    pub fn new(max_rounds: u32) -> Self {
        Self { max_rounds }
    }

    /// 创建默认法庭
    pub fn default_courtroom() -> Self {
        Self::new(5)
    }

    /// 执行审议
    pub async fn deliberate(&self, task: &str) -> nl_core::Result<Verdict> {
        // TODO: 实现实际的审议逻辑
        Ok(Verdict::approved(Uuid::new_v4(), 0.8, "Default approval"))
    }
}

impl Default for Courtroom {
    fn default() -> Self {
        Self::default_courtroom()
    }
}
