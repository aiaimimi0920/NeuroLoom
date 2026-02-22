//! Provider Trait 定义

use async_trait::async_trait;
use futures::Stream;

use crate::primitive::PrimitiveRequest;
use crate::auth::Auth;

/// LLM Provider 统一 Trait
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Provider 唯一标识
    fn id(&self) -> &str;

    /// 认证类型
    fn auth(&self) -> &Auth;

    /// 支持的模型列表
    fn supported_models(&self) -> &[&str];

    /// 将原语编译为请求体
    fn compile(&self, primitive: &PrimitiveRequest) -> serde_json::Value;

    /// 执行请求
    async fn complete(&self, body: serde_json::Value) -> crate::Result<LlmResponse>;

    /// 流式执行
    async fn stream(
        &self,
        body: serde_json::Value,
    ) -> crate::Result<BoxStream<'_, crate::Result<LlmChunk>>>;

    /// 是否需要刷新认证
    fn needs_refresh(&self) -> bool {
        false
    }

    /// 刷新认证
    async fn refresh_auth(&mut self) -> crate::Result<()> {
        Ok(())
    }
}

/// LLM 响应
#[derive(Debug, Clone)]
pub struct LlmResponse {
    /// 响应内容
    pub content: String,
    /// 工具调用
    pub tool_calls: Vec<ToolCall>,
    /// 使用统计
    pub usage: Usage,
    /// 停止原因
    pub stop_reason: StopReason,
}

/// LLM 流式块
#[derive(Debug, Clone)]
pub struct LlmChunk {
    /// 增量内容
    pub delta: ChunkDelta,
    /// 使用统计（最后一块可能有）
    pub usage: Option<Usage>,
}

/// 增量内容类型
#[derive(Debug, Clone)]
pub enum ChunkDelta {
    /// 文本
    Text(String),
    /// 工具调用
    ToolCall {
        id: String,
        name: String,
        delta: String,
    },
    /// 思考内容
    Thinking(String),
}

/// 停止原因
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// 正常结束
    EndTurn,
    /// 工具调用
    ToolUse,
    /// 达到最大 token
    MaxTokens,
    /// 遇到停止序列
    StopSequence,
}

/// 工具调用
#[derive(Debug, Clone)]
pub struct ToolCall {
    /// 调用 ID
    pub id: String,
    /// 工具名称
    pub name: String,
    /// 参数
    pub arguments: serde_json::Value,
}

/// 使用统计
#[derive(Debug, Clone, Default)]
pub struct Usage {
    /// 输入 token 数
    pub input_tokens: u64,
    /// 输出 token 数
    pub output_tokens: u64,
    /// 思考 token 数（如果有）
    pub thinking_tokens: Option<u64>,
}

/// Provider 执行错误，带有重试信号
#[derive(Debug, Clone)]
pub struct ProviderError {
    /// 错误消息
    pub message: String,
    /// 是否应该在同一 Provider 重试
    pub retryable: bool,
    /// 是否应该触发跨 Provider 降级
    pub should_fallback: bool,
    /// 建议的重试延迟（毫秒）
    pub retry_after_ms: Option<u64>,
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ProviderError {}

impl From<ProviderError> for crate::Error {
    fn from(e: ProviderError) -> Self {
        crate::Error::Provider(e.message)
    }
}

/// BoxStream 类型别名
pub type BoxStream<'a, T> = std::pin::Pin<Box<dyn Stream<Item = T> + Send + 'a>>;
