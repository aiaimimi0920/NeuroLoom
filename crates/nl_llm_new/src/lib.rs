//! nl_llm 模块入口
//!
//! LLM Gateway 核心模块，提供：
//! - 多协议 Provider 抽象
//! - 原语格式中间层
//! - 格式转换管道
//! - 认证管理
//! - 黑魔法代理聚合
//! - 分层容错机制

pub mod auth;
pub mod primitive;
pub mod translator;
pub mod black_magic_proxy;
pub mod provider;
pub mod gateway;
pub mod fallback;
pub mod token_bucket;

// 重导出常用类型
pub use primitive::{PrimitiveRequest, PrimitiveMessage, PrimitiveContent, PrimitiveTool};
pub use translator::{Format, WrapperKind, TranslatorPipeline, TranslateError};
pub use auth::{Auth, ApiKeyConfig, OAuthProvider, SAProvider, TokenStorage, TokenStatus};
pub use gateway::{Gateway, GatewayConfig, GatewayError};
pub use provider::{LlmProvider, LlmResponse, ProviderError};

/// 模块错误类型
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("认证错误: {0}")]
    Auth(String),

    #[error("Provider 错误: {0}")]
    Provider(#[from] crate::provider::ProviderError),

    #[error("转换错误: {0}")]
    Translate(#[from] TranslateError),

    #[error("Gateway 错误: {0}")]
    Gateway(#[from] GatewayError),

    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON 错误: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP 错误: {0}")]
    Http(String),

    #[error("未知错误: {0}")]
    Unknown(String),
}

pub type Result<T> = std::result::Result<T, Error>;
