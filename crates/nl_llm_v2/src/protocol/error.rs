/// 错误类型枚举
///
/// 涵盖 LLM API 调用过程中可能遇到的所有错误类型，
/// 用于错误分类和降级决策。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// 认证错误（401/403）
    /// API Key 无效或已过期
    Authentication,

    /// 配额超限（429）
    /// 请求频率超过限制，需要等待或重试
    RateLimit,

    /// 模型不可用
    /// 指定的模型不存在或已下线
    ModelUnavailable,

    /// 上下文超长
    /// 输入 token 数超过模型上下文窗口限制
    ContextLengthExceeded,

    /// 内容过滤
    /// 输入或输出触发了内容安全策略
    ContentFilter,

    /// 服务端错误（500+）
    /// API 服务端内部错误
    ServerError,

    /// 网络错误
    /// 连接失败、DNS 解析失败等网络层面的问题
    Network,

    /// 超时错误
    /// 请求超时，与 ServerError 区分（可能是客户端设置的超时）
    Timeout,

    /// 其他错误
    /// 未分类的错误类型
    Other,
}

/// 降级建议
///
/// 根据错误类型提供可操作的降级或重试建议，
/// 帮助上层系统做出智能决策。
#[derive(Debug, Clone)]
pub enum FallbackHint {
    /// 重试当前平台
    /// 适用于临时性错误（如 RateLimit、ServerError）
    Retry,

    /// 降级到其他平台
    /// 参数为目标平台名称
    FallbackTo(String),

    /// 降低模型规格
    /// 尝试使用更小/更便宜的模型
    DowngradeModel,

    /// 无建议
    /// 错误无法自动恢复，需要人工介入
    None,
}

/// 标准错误类型
///
/// 统一的错误表示，包含错误分类、可重试性判断和降级建议。
/// 所有平台的错误都应转换为此格式，便于上层统一处理。
///
/// # 示例
///
/// ```
/// use nl_llm_v2::protocol::error::{StandardError, ErrorKind, FallbackHint};
///
/// let error = StandardError {
///     kind: ErrorKind::RateLimit,
///     message: "Rate limit exceeded".to_string(),
///     code: Some("429".to_string()),
///     retryable: true,
///     fallback_hint: Some(FallbackHint::Retry),
/// };
///
/// assert!(error.retryable);
/// ```
#[derive(Debug, Clone)]
pub struct StandardError {
    /// 错误类型
    pub kind: ErrorKind,

    /// 错误消息
    pub message: String,

    /// 原始错误码（如有）
    /// 如 OpenAI 的 "context_length_exceeded"、Claude 的 "invalid_request_error"
    pub code: Option<String>,

    /// 是否可重试
    /// 为 true 时建议等待后重试
    pub retryable: bool,

    /// 建议的降级动作
    /// 提供可操作的恢复建议
    pub fallback_hint: Option<FallbackHint>,
}

impl std::fmt::Display for StandardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:?}] {}", self.kind, self.message)?;
        if let Some(c) = &self.code {
            write!(f, " (Code: {})", c)?;
        }
        Ok(())
    }
}

impl std::error::Error for StandardError {}

impl StandardError {
    /// 创建认证错误
    pub fn authentication(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Authentication,
            message: message.into(),
            code: None,
            retryable: false,
            fallback_hint: None,
        }
    }

    /// 创建配额超限错误
    pub fn rate_limit(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::RateLimit,
            message: message.into(),
            code: None,
            retryable: true,
            fallback_hint: Some(FallbackHint::Retry),
        }
    }

    /// 创建网络错误
    pub fn network(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Network,
            message: message.into(),
            code: None,
            retryable: true,
            fallback_hint: Some(FallbackHint::Retry),
        }
    }

    /// 创建超时错误
    pub fn timeout(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Timeout,
            message: message.into(),
            code: None,
            retryable: true,
            fallback_hint: Some(FallbackHint::Retry),
        }
    }

    /// 创建服务端错误
    pub fn server_error(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::ServerError,
            message: message.into(),
            code: None,
            retryable: true,
            fallback_hint: Some(FallbackHint::Retry),
        }
    }

    /// 创建模型不可用错误
    pub fn model_unavailable(message: impl Into<String>, model: &str) -> Self {
        Self {
            kind: ErrorKind::ModelUnavailable,
            message: message.into(),
            code: Some(model.to_string()),
            retryable: false,
            fallback_hint: Some(FallbackHint::DowngradeModel),
        }
    }

    /// 创建上下文超长错误
    pub fn context_too_long(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::ContextLengthExceeded,
            message: message.into(),
            code: None,
            retryable: false,
            fallback_hint: Some(FallbackHint::DowngradeModel),
        }
    }
}

impl From<anyhow::Error> for StandardError {
    fn from(err: anyhow::Error) -> Self {
        let err_str = err.to_string().to_lowercase();

        // 根据错误消息内容推断错误类型
        let kind = if err_str.contains("429")
            || err_str.contains("rate limit")
            || err_str.contains("too many requests")
        {
            ErrorKind::RateLimit
        } else if err_str.contains("401")
            || err_str.contains("403")
            || err_str.contains("unauthorized")
            || err_str.contains("forbidden")
        {
            ErrorKind::Authentication
        } else if err_str.contains("timeout") || err_str.contains("timed out") {
            ErrorKind::Timeout
        } else if err_str.contains("connection")
            || err_str.contains("network")
            || err_str.contains("dns")
        {
            ErrorKind::Network
        } else if err_str.contains("500") || err_str.contains("502") || err_str.contains("503") {
            ErrorKind::ServerError
        } else if err_str.contains("context") && err_str.contains("length") {
            ErrorKind::ContextLengthExceeded
        } else {
            ErrorKind::Other
        };

        let retryable = matches!(
            kind,
            ErrorKind::RateLimit | ErrorKind::ServerError | ErrorKind::Network | ErrorKind::Timeout
        );
        let fallback_hint = if retryable {
            Some(FallbackHint::Retry)
        } else {
            None
        };

        Self {
            kind,
            message: err.to_string(),
            code: None,
            retryable,
            fallback_hint,
        }
    }
}
