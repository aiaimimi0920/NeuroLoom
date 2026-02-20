//! 核心实体定义

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 实体 ID 类型别名
pub type EntityId = Uuid;

/// 基础实体特征
pub trait Entity: Send + Sync + 'static {
    /// 获取实体 ID
    fn id(&self) -> EntityId;

    /// 获取实体类型
    fn entity_type(&self) -> &'static str;
}

/// 工作区节点实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceNode {
    /// 节点 ID
    pub id: EntityId,
    /// 节点标题
    pub title: String,
    /// 节点内容
    pub content: String,
    /// 节点类型
    pub node_type: NodeType,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 更新时间
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// 是否折叠
    pub is_collapsed: bool,
    /// 输入连接 (上游节点 ID)
    pub inputs: Vec<EntityId>,
    /// 输出连接 (下游节点 ID)
    pub outputs: Vec<EntityId>,
}

impl Entity for WorkspaceNode {
    fn id(&self) -> EntityId {
        self.id
    }

    fn entity_type(&self) -> &'static str {
        "workspace_node"
    }
}

/// 节点类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NodeType {
    /// 静态工作区
    Workspace,
    /// 活体流式节点
    LiveStream,
    /// 预生成期约节点
    Promise,
    /// 桌面贴纸
    Sticker,
    /// 状态胶囊
    Capsule,
}

impl WorkspaceNode {
    /// 创建新的工作区节点
    pub fn new(title: impl Into<String>, node_type: NodeType) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4(),
            title: title.into(),
            content: String::new(),
            node_type,
            created_at: now,
            updated_at: now,
            is_collapsed: false,
            inputs: Vec::new(),
            outputs: Vec::new(),
        }
    }

    /// 添加输入连接
    pub fn add_input(&mut self, input_id: EntityId) {
        if !self.inputs.contains(&input_id) {
            self.inputs.push(input_id);
        }
    }

    /// 添加输出连接
    pub fn add_output(&mut self, output_id: EntityId) {
        if !self.outputs.contains(&output_id) {
            self.outputs.push(output_id);
        }
    }
}
