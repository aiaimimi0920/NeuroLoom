//! HAP 协议定义

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// HAP 消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HapMessage {
    /// 消息 ID
    pub id: Uuid,
    /// 消息类型
    pub msg_type: HapMessageType,
    /// 发送者 ID
    pub sender: Uuid,
    /// 接收者 ID (None 表示广播)
    pub receiver: Option<Uuid>,
    /// 时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// 载荷
    pub payload: serde_json::Value,
}

impl HapMessage {
    /// 创建新消息
    pub fn new(msg_type: HapMessageType, sender: Uuid, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            msg_type,
            sender,
            receiver: None,
            timestamp: chrono::Utc::now(),
            payload,
        }
    }

    /// 设置接收者
    pub fn to(mut self, receiver: Uuid) -> Self {
        self.receiver = Some(receiver);
        self
    }

    /// 序列化为 JSON
    pub fn to_json(&self) -> nl_core::Result<String> {
        serde_json::to_string(self).map_err(|e| nl_core::NeuroLoomError::Serialization(e))
    }

    /// 从 JSON 反序列化
    pub fn from_json(json: &str) -> nl_core::Result<Self> {
        serde_json::from_str(json).map_err(|e| nl_core::NeuroLoomError::Serialization(e))
    }
}

/// HAP 消息类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HapMessageType {
    /// 握手请求
    Handshake,
    /// 握手响应
    HandshakeAck,
    /// 心跳
    Heartbeat,
    /// 心跳响应
    HeartbeatAck,
    /// 任务广播
    TaskBroadcast,
    /// 竞标
    Bid,
    /// 竞标响应
    BidAck,
    /// 任务分配
    TaskAssign,
    /// 任务结果
    TaskResult,
    /// 错误
    Error,
    /// 自定义
    Custom(String),
}

/// HAP 协议处理器
pub struct HapProtocol;

impl HapProtocol {
    /// 创建握手消息
    pub fn handshake(agent_id: Uuid, capabilities: Vec<String>) -> HapMessage {
        HapMessage::new(
            HapMessageType::Handshake,
            agent_id,
            serde_json::json!({ "capabilities": capabilities }),
        )
    }

    /// 创建任务广播
    pub fn task_broadcast(agent_id: Uuid, task: &str, requirements: Vec<String>) -> HapMessage {
        HapMessage::new(
            HapMessageType::TaskBroadcast,
            agent_id,
            serde_json::json!({
                "task": task,
                "requirements": requirements,
            }),
        )
    }

    /// 创建竞标消息
    pub fn bid(agent_id: Uuid, task_id: Uuid, price: f64, eta_secs: u64) -> HapMessage {
        HapMessage::new(
            HapMessageType::Bid,
            agent_id,
            serde_json::json!({
                "task_id": task_id,
                "price": price,
                "eta_secs": eta_secs,
            }),
        )
    }

    /// 创建任务结果
    pub fn task_result(agent_id: Uuid, task_id: Uuid, result: &str) -> HapMessage {
        HapMessage::new(
            HapMessageType::TaskResult,
            agent_id,
            serde_json::json!({
                "task_id": task_id,
                "result": result,
            }),
        )
    }
}
