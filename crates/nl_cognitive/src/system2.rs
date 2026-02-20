//! 认知引擎 - MCTS 引擎
//!
//! System 2: 蒙特卡洛树搜索，自适应算力推演。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use nl_core::Result;

/// MCTS 节点
#[derive(Debug, Clone)]
pub struct MctsNode {
    /// 节点 ID
    pub id: Uuid,
    /// 状态
    pub state: String,
    /// 父节点
    pub parent: Option<Uuid>,
    /// 子节点
    pub children: Vec<Uuid>,
    /// 访问次数
    pub visits: u32,
    /// 累计奖励
    pub total_reward: f64,
    /// 是否终止状态
    pub is_terminal: bool,
}

impl MctsNode {
    /// 创建新节点
    pub fn new(state: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            state: state.into(),
            parent: None,
            children: Vec::new(),
            visits: 0,
            total_reward: 0.0,
            is_terminal: false,
        }
    }

    /// 计算 UCB1 值
    pub fn ucb1(&self, exploration_constant: f64, parent_visits: u32) -> f64 {
        if self.visits == 0 {
            return f64::INFINITY;
        }
        let exploitation = self.total_reward / self.visits as f64;
        let exploration = exploration_constant * (parent_visits as f64).ln().sqrt() / self.visits as f64;
        exploitation + exploration
    }

    /// 平均奖励
    pub fn average_reward(&self) -> f64 {
        if self.visits == 0 {
            0.0
        } else {
            self.total_reward / self.visits as f64
        }
    }
}

/// MCTS 配置
#[derive(Debug, Clone)]
pub struct MctsConfig {
    /// 探索常数
    pub exploration_constant: f64,
    /// 最大迭代次数
    pub max_iterations: u32,
    /// 最大深度
    pub max_depth: u32,
    /// 早停阈值 (当奖励超过此值时停止)
    pub early_stop_threshold: f64,
    /// 挫败指数阈值 (连续失败多少次触发熔断)
    pub frustration_threshold: u32,
}

impl Default for MctsConfig {
    fn default() -> Self {
        Self {
            exploration_constant: 1.414,
            max_iterations: 1000,
            max_depth: 10,
            early_stop_threshold: 0.95,
            frustration_threshold: 50,
        }
    }
}

/// MCTS 引擎
pub struct MctsEngine {
    /// 配置
    config: MctsConfig,
    /// 所有节点
    nodes: HashMap<Uuid, MctsNode>,
    /// 根节点
    root: Option<Uuid>,
    /// 挫败计数器
    frustration_count: u32,
}

impl MctsEngine {
    /// 创建新的 MCTS 引擎
    pub fn new(config: MctsConfig) -> Self {
        Self {
            config,
            nodes: HashMap::new(),
            root: None,
            frustration_count: 0,
        }
    }

    /// 创建默认配置的引擎
    pub fn default_engine() -> Self {
        Self::new(MctsConfig::default())
    }

    /// 设置根状态
    pub fn set_root(&mut self, state: impl Into<String>) {
        let mut root = MctsNode::new(state);
        root.id = Uuid::nil(); // 根节点使用 nil UUID
        self.root = Some(root.id);
        self.nodes.insert(root.id, root);
    }

    /// 执行 MCTS 搜索
    pub async fn search(&mut self) -> Result<Option<String>> {
        let root_id = self.root.ok_or_else(|| {
            nl_core::NeuroLoomError::Unknown("Root not set".to_string())
        })?;

        for iteration in 0..self.config.max_iterations {
            // 选择
            let selected = self.select(root_id)?;

            // 扩展
            let expanded = self.expand(selected).await?;

            // 模拟
            let reward = self.simulate(expanded).await?;

            // 回溯
            self.backpropagate(expanded, reward);

            // 早停检查
            if let Some(node) = self.nodes.get(&root_id) {
                if node.average_reward() >= self.config.early_stop_threshold {
                    return Ok(self.best_action());
                }
            }

            // 挫败检查
            if self.frustration_count >= self.config.frustration_threshold {
                break;
            }
        }

        Ok(self.best_action())
    }

    /// 选择阶段
    fn select(&self, from: Uuid) -> Result<Uuid> {
        let mut current = from;

        loop {
            let node = self.nodes.get(&current).ok_or_else(|| {
                nl_core::NeuroLoomError::Unknown(format!("Node not found: {}", current))
            })?;

            if node.children.is_empty() || node.is_terminal {
                return Ok(current);
            }

            // 选择 UCB1 最大的子节点
            let best_child = node
                .children
                .iter()
                .filter_map(|id| self.nodes.get(id).map(|n| (id, n)))
                .max_by(|(_, a), (_, b)| {
                    a.ucb1(self.config.exploration_constant, node.visits)
                        .partial_cmp(&b.ucb1(self.config.exploration_constant, node.visits))
                        .unwrap()
                })
                .map(|(id, _)| *id);

            current = best_child.unwrap_or(node.children[0]);
        }
    }

    /// 扩展阶段
    async fn expand(&mut self, node_id: Uuid) -> Result<Uuid> {
        // TODO: 实现实际的扩展逻辑
        Ok(node_id)
    }

    /// 模拟阶段
    async fn simulate(&mut self, node_id: Uuid) -> Result<f64> {
        // TODO: 实现实际的模拟逻辑
        Ok(0.5)
    }

    /// 回溯阶段
    fn backpropagate(&mut self, node_id: Uuid, reward: f64) {
        let mut current = Some(node_id);

        while let Some(id) = current {
            if let Some(node) = self.nodes.get_mut(&id) {
                node.visits += 1;
                node.total_reward += reward;
                current = node.parent;
            } else {
                break;
            }
        }
    }

    /// 获取最佳动作
    fn best_action(&self) -> Option<String> {
        self.root.and_then(|root_id| {
            self.nodes.get(&root_id).and_then(|root| {
                root.children
                    .iter()
                    .filter_map(|id| self.nodes.get(id))
                    .max_by(|a, b| a.visits.cmp(&b.visits))
                    .map(|n| n.state.clone())
            })
        })
    }

    /// 重置引擎
    pub fn reset(&mut self) {
        self.nodes.clear();
        self.root = None;
        self.frustration_count = 0;
    }
}
