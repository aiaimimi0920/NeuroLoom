//! 认知引擎 - SOP 引擎
//!
//! System 1: 高频任务 DAG 工作流固化引擎。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use nl_core::Result;

/// SOP 节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SopNode {
    /// 节点 ID
    pub id: Uuid,
    /// 节点名称
    pub name: String,
    /// 执行动作
    pub action: SopAction,
    /// 下游节点
    pub next: Vec<Uuid>,
    /// 失败处理
    pub on_failure: Option<Uuid>,
}

/// SOP 动作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SopAction {
    /// 执行命令
    ExecuteCommand { command: String, args: Vec<String> },
    /// 调用 LLM
    CallLLM { prompt: String, model: String },
    /// 条件分支
    Condition { expression: String, true_branch: Uuid, false_branch: Uuid },
    /// 并行执行
    Parallel { branches: Vec<Uuid> },
    /// 等待
    Wait { seconds: u64 },
    /// 自定义脚本
    Script { language: String, code: String },
}

/// SOP 工作流
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SopWorkflow {
    /// 工作流 ID
    pub id: Uuid,
    /// 工作流名称
    pub name: String,
    /// 描述
    pub description: String,
    /// 入口节点
    pub entry: Uuid,
    /// 所有节点
    pub nodes: HashMap<Uuid, SopNode>,
    /// 变量
    pub variables: HashMap<String, String>,
}

impl SopWorkflow {
    /// 创建新的工作流
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            description: String::new(),
            entry: Uuid::nil(),
            nodes: HashMap::new(),
            variables: HashMap::new(),
        }
    }

    /// 添加节点
    pub fn add_node(&mut self, node: SopNode) {
        self.nodes.insert(node.id, node);
    }

    /// 设置入口
    pub fn set_entry(&mut self, entry: Uuid) {
        self.entry = entry;
    }

    /// 获取节点
    pub fn get_node(&self, id: &Uuid) -> Option<&SopNode> {
        self.nodes.get(id)
    }
}

/// SOP 执行上下文
#[derive(Debug, Clone)]
pub struct SopContext {
    /// 当前节点 ID
    pub current_node: Uuid,
    /// 变量
    pub variables: HashMap<String, String>,
    /// 执行历史
    pub history: Vec<Uuid>,
    /// 执行结果
    pub results: HashMap<Uuid, String>,
}

/// SOP 引擎
pub struct SopEngine {
    /// 已注册的工作流
    workflows: HashMap<Uuid, SopWorkflow>,
    /// 名称索引
    name_index: HashMap<String, Uuid>,
}

impl SopEngine {
    /// 创建新的 SOP 引擎
    pub fn new() -> Self {
        Self {
            workflows: HashMap::new(),
            name_index: HashMap::new(),
        }
    }

    /// 注册工作流
    pub fn register(&mut self, workflow: SopWorkflow) {
        self.name_index.insert(workflow.name.clone(), workflow.id);
        self.workflows.insert(workflow.id, workflow);
    }

    /// 通过名称查找工作流
    pub fn find(&self, name: &str) -> Option<&SopWorkflow> {
        self.name_index
            .get(name)
            .and_then(|id| self.workflows.get(id))
    }

    /// 执行工作流
    pub async fn execute(&self, workflow_id: &Uuid) -> Result<SopContext> {
        let workflow = self.workflows.get(workflow_id).ok_or_else(|| {
            nl_core::NeuroLoomError::Unknown(format!("Workflow not found: {}", workflow_id))
        })?;

        let mut ctx = SopContext {
            current_node: workflow.entry,
            variables: workflow.variables.clone(),
            history: Vec::new(),
            results: HashMap::new(),
        };

        // 执行工作流
        while let Some(node) = workflow.get_node(&ctx.current_node) {
            ctx.history.push(ctx.current_node);

            // 执行动作
            let result = self.execute_action(&node.action, &ctx).await?;
            ctx.results.insert(ctx.current_node, result);

            // 移动到下一个节点
            if node.next.is_empty() {
                break;
            }
            ctx.current_node = node.next[0];
        }

        Ok(ctx)
    }

    /// 执行单个动作
    async fn execute_action(&self, action: &SopAction, ctx: &SopContext) -> Result<String> {
        match action {
            SopAction::ExecuteCommand { command, args } => {
                // TODO: 实现命令执行
                Ok(format!("Executed: {} {:?}", command, args))
            }
            SopAction::CallLLM { prompt, model } => {
                // TODO: 实现 LLM 调用
                Ok(format!("Called LLM: {} with {}", model, prompt))
            }
            SopAction::Wait { seconds } => {
                tokio::time::sleep(tokio::time::Duration::from_secs(*seconds)).await;
                Ok(format!("Waited {} seconds", seconds))
            }
            _ => Ok("Action completed".to_string()),
        }
    }

    /// 获取工作流数量
    pub fn count(&self) -> usize {
        self.workflows.len()
    }
}

impl Default for SopEngine {
    fn default() -> Self {
        Self::new()
    }
}
