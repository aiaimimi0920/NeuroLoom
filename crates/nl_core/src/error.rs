//! 全局错误处理机制

use thiserror::Error;

/// NeuroLoom 统一错误类型
#[derive(Error, Debug)]
pub enum NeuroLoomError {
    #[error("Event store error: {0}")]
    EventStore(String),

    #[error("LLM provider error: {0}")]
    LlmProvider(String),

    #[error("Token bucket exhausted: {0}")]
    TokenBucketExhausted(String),

    #[error("Actor error: {0}")]
    Actor(String),

    #[error("Memory error: {0}")]
    Memory(String),

    #[error("Sandbox execution error: {0}")]
    Sandbox(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// 统一 Result 类型别名
pub type Result<T> = std::result::Result<T, NeuroLoomError>;
