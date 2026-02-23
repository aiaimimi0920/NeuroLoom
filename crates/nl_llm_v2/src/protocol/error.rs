#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// 认证错误（401/403）
    Authentication,
    /// 配额超限（429）
    RateLimit,
    /// 模型不可用
    ModelUnavailable,
    /// 上下文超长
    ContextLengthExceeded,
    /// 内容过滤
    ContentFilter,
    /// 服务端错误（500+）
    ServerError,
    /// 其他错误
    Other,
}

#[derive(Debug, Clone)]
pub enum FallbackHint {
    /// 重试当前平台
    Retry,
    /// 降级到其他平台
    FallbackTo(String),
    /// 降低模型规格
    DowngradeModel,
    /// 无建议
    None,
}

/// 标准错误类型
#[derive(Debug, Clone)]
pub struct StandardError {
    /// 错误类型
    pub kind: ErrorKind,
    /// 错误消息
    pub message: String,
    /// 原始错误码（如有）
    pub code: Option<String>,
    /// 是否可重试
    pub retryable: bool,
    /// 建议的降级动作
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
