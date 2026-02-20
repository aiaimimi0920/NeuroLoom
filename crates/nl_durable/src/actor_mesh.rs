//! Actor Mesh - Actor 生命周期管理

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

use nl_core::event::Event;
use nl_core::entity::EntityId;
use nl_core::Result;

/// Actor ID
pub type ActorId = Uuid;

/// Actor 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActorState {
    /// 正在运行
    Running,
    /// 已暂停
    Suspended,
    /// 已休眠 (持久化)
    Hibernated,
    /// 已终止
    Terminated,
}

/// Actor 地址
#[derive(Debug, Clone)]
pub struct ActorAddress {
    pub id: ActorId,
    pub sender: mpsc::Sender<ActorMessage>,
}

/// Actor 消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActorMessage {
    /// 处理事件
    ProcessEvent(Event),
    /// 暂停
    Suspend,
    /// 唤醒
    Resume,
    /// 休眠
    Hibernate,
    /// 终止
    Terminate,
    /// 自定义消息
    Custom(String, serde_json::Value),
}

/// Actor 特征
#[async_trait]
pub trait Actor: Send + Sync + 'static {
    /// Actor 类型名称
    fn type_name(&self) -> &'static str;

    /// 处理消息
    async fn handle(&mut self, msg: ActorMessage) -> Result<Option<Event>>;

    /// 恢复状态 (从事件流重放)
    async fn recover(&mut self, events: Vec<Event>) -> Result<()>;

    /// 获取当前状态
    fn state(&self) -> ActorState;
}

/// Actor 上下文
pub struct ActorContext {
    pub id: ActorId,
    pub state: ActorState,
    pub mailbox: mpsc::Receiver<ActorMessage>,
}

/// Actor Mesh - 管理所有 Actor
pub struct ActorMesh {
    actors: Arc<RwLock<HashMap<ActorId, ActorAddress>>>,
    states: Arc<RwLock<HashMap<ActorId, ActorState>>>,
}

impl ActorMesh {
    /// 创建新的 Actor Mesh
    pub fn new() -> Self {
        Self {
            actors: Arc::new(RwLock::new(HashMap::new())),
            states: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 生成 Actor ID
    pub fn generate_id() -> ActorId {
        Uuid::new_v4()
    }

    /// 注册 Actor
    pub async fn register(&self, id: ActorId, address: ActorAddress) {
        let mut actors = self.actors.write().await;
        actors.insert(id, address);

        let mut states = self.states.write().await;
        states.insert(id, ActorState::Running);
    }

    /// 注销 Actor
    pub async fn unregister(&self, id: &ActorId) {
        let mut actors = self.actors.write().await;
        actors.remove(id);

        let mut states = self.states.write().await;
        states.remove(id);
    }

    /// 发送消息到 Actor
    pub async fn send(&self, id: &ActorId, msg: ActorMessage) -> Result<()> {
        let actors = self.actors.read().await;
        if let Some(addr) = actors.get(id) {
            addr.sender
                .send(msg)
                .await
                .map_err(|e| nl_core::NeuroLoomError::Actor(e.to_string()))?;
        }
        Ok(())
    }

    /// 获取 Actor 状态
    pub async fn get_state(&self, id: &ActorId) -> Option<ActorState> {
        let states = self.states.read().await;
        states.get(id).copied()
    }

    /// 更新 Actor 状态
    pub async fn set_state(&self, id: &ActorId, state: ActorState) {
        let mut states = self.states.write().await;
        states.insert(*id, state);
    }

    /// 获取所有 Actor ID
    pub async fn all_actors(&self) -> Vec<ActorId> {
        let actors = self.actors.read().await;
        actors.keys().copied().collect()
    }

    /// 获取 Actor 数量
    pub async fn count(&self) -> usize {
        let actors = self.actors.read().await;
        actors.len()
    }
}

impl Default for ActorMesh {
    fn default() -> Self {
        Self::new()
    }
}
