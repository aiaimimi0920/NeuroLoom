//! Agent 市场

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 竞标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bid {
    /// 竞标 ID
    pub id: Uuid,
    /// Agent ID
    pub agent_id: Uuid,
    /// 任务 ID
    pub task_id: Uuid,
    /// 价格
    pub price: f64,
    /// 预计完成时间 (秒)
    pub eta_secs: u64,
    /// 能力评分
    pub capability_score: f64,
}

impl Bid {
    pub fn new(agent_id: Uuid, task_id: Uuid, price: f64, eta_secs: u64) -> Self {
        Self {
            id: Uuid::new_v4(),
            agent_id,
            task_id,
            price,
            eta_secs,
            capability_score: 1.0,
        }
    }
}

/// 任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// 任务 ID
    pub id: Uuid,
    /// 描述
    pub description: String,
    /// 要求
    pub requirements: Vec<String>,
    /// 预算
    pub budget: Option<f64>,
    /// 状态
    pub status: TaskStatus,
    /// 分配给的 Agent
    pub assigned_to: Option<Uuid>,
}

/// 任务状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    Open,
    Bidding,
    Assigned,
    InProgress,
    Completed,
    Cancelled,
}

/// Agent 市场
pub struct AgentMarket {
    /// 开放任务
    open_tasks: HashMap<Uuid, Task>,
    /// 竞标
    bids: HashMap<Uuid, Vec<Bid>>,
    /// Agent 评分
    agent_scores: HashMap<Uuid, f64>,
}

impl AgentMarket {
    /// 创建新市场
    pub fn new() -> Self {
        Self {
            open_tasks: HashMap::new(),
            bids: HashMap::new(),
            agent_scores: HashMap::new(),
        }
    }

    /// 发布任务
    pub fn publish_task(&mut self, task: Task) {
        self.open_tasks.insert(task.id, task);
    }

    /// 提交竞标
    pub fn submit_bid(&mut self, bid: Bid) {
        self.bids
            .entry(bid.task_id)
            .or_default()
            .push(bid);
    }

    /// 选择最佳竞标
    pub fn select_best_bid(&self, task_id: &Uuid) -> Option<&Bid> {
        self.bids
            .get(task_id)?
            .iter()
            .min_by(|a, b| {
                let score_a = a.price + (a.eta_secs as f64 * 0.01);
                let score_b = b.price + (b.eta_secs as f64 * 0.01);
                score_a.partial_cmp(&score_b).unwrap()
            })
    }

    /// 分配任务
    pub fn assign_task(&mut self, task_id: &Uuid, agent_id: Uuid) -> Option<&Task> {
        if let Some(task) = self.open_tasks.get_mut(task_id) {
            task.status = TaskStatus::Assigned;
            task.assigned_to = Some(agent_id);
            Some(task)
        } else {
            None
        }
    }

    /// 获取开放任务
    pub fn open_tasks(&self) -> Vec<&Task> {
        self.open_tasks
            .values()
            .filter(|t| t.status == TaskStatus::Open)
            .collect()
    }

    /// 更新 Agent 评分
    pub fn update_agent_score(&mut self, agent_id: Uuid, score: f64) {
        self.agent_scores.insert(agent_id, score);
    }

    /// 获取 Agent 评分
    pub fn get_agent_score(&self, agent_id: &Uuid) -> f64 {
        self.agent_scores.get(agent_id).copied().unwrap_or(1.0)
    }
}

impl Default for AgentMarket {
    fn default() -> Self {
        Self::new()
    }
}
