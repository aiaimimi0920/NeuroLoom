//! 事件溯源核心事件定义

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entity::EntityId;

/// 事件溯源事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// 事件唯一标识
    pub id: Uuid,
    /// 事件类型
    pub kind: EventKind,
    /// 事件时间戳
    pub timestamp: DateTime<Utc>,
    /// 关联实体 ID
    pub entity_id: EntityId,
    /// 事件载荷 (JSON)
    pub payload: serde_json::Value,
    /// 因果关系: 前置事件 ID
    pub causation_id: Option<Uuid>,
    /// 关联关系: 触发此事件的命令 ID
    pub correlation_id: Option<Uuid>,
}

impl Event {
    /// 创建新事件
    pub fn new(kind: EventKind, entity_id: EntityId, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            kind,
            timestamp: Utc::now(),
            entity_id,
            payload,
            causation_id: None,
            correlation_id: None,
        }
    }

    /// 设置因果关系
    pub fn with_causation(mut self, causation_id: Uuid) -> Self {
        self.causation_id = Some(causation_id);
        self
    }

    /// 设置关联关系
    pub fn with_correlation(mut self, correlation_id: Uuid) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }
}

/// 事件类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventKind {
    // 工作区事件
    NodeCreated,
    NodeUpdated,
    NodeDeleted,

    // LLM 事件
    LlmRequestStarted,
    LlmResponseChunk,
    LlmResponseCompleted,
    LlmError,

    // 执行事件
    CodeExecuted,
    ExecutionFailed,
    ExecutionSuccess,

    // 认知事件
    TaskAssigned,
    TaskCompleted,
    VerdictIssued,

    // Actor 事件
    ActorSpawned,
    ActorSuspended,
    ActorResumed,
    ActorTerminated,

    // 记忆事件
    MemoryStored,
    MemoryRetrieved,
    MemoryArchived,

    // 网络事件
    AgentConnected,
    AgentDisconnected,
    BidReceived,
    TaskDelegated,

    // 自定义事件
    Custom(String),
}

impl EventKind {
    /// 获取事件类型名称
    pub fn as_str(&self) -> &str {
        match self {
            EventKind::NodeCreated => "node_created",
            EventKind::NodeUpdated => "node_updated",
            EventKind::NodeDeleted => "node_deleted",
            EventKind::LlmRequestStarted => "llm_request_started",
            EventKind::LlmResponseChunk => "llm_response_chunk",
            EventKind::LlmResponseCompleted => "llm_response_completed",
            EventKind::LlmError => "llm_error",
            EventKind::CodeExecuted => "code_executed",
            EventKind::ExecutionFailed => "execution_failed",
            EventKind::ExecutionSuccess => "execution_success",
            EventKind::TaskAssigned => "task_assigned",
            EventKind::TaskCompleted => "task_completed",
            EventKind::VerdictIssued => "verdict_issued",
            EventKind::ActorSpawned => "actor_spawned",
            EventKind::ActorSuspended => "actor_suspended",
            EventKind::ActorResumed => "actor_resumed",
            EventKind::ActorTerminated => "actor_terminated",
            EventKind::MemoryStored => "memory_stored",
            EventKind::MemoryRetrieved => "memory_retrieved",
            EventKind::MemoryArchived => "memory_archived",
            EventKind::AgentConnected => "agent_connected",
            EventKind::AgentDisconnected => "agent_disconnected",
            EventKind::BidReceived => "bid_received",
            EventKind::TaskDelegated => "task_delegated",
            EventKind::Custom(name) => name,
        }
    }
}
