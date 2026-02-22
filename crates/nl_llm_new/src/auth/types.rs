//! 认证类型定义
//!
//! 定义三种认证类型的统一抽象：
//! - API Key：统一结构，通过 base_url 区分官方/转发站
//! - OAuth：各 Provider 独立实现
//! - Service Account：GCP 专用

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;

/// 认证类型（顶层枚举）
#[derive(Debug, Clone)]
pub enum Auth {
    /// API Key 认证
    ApiKey(ApiKeyConfig),

    /// OAuth 认证
    OAuth {
        provider: OAuthProvider,
        token_path: PathBuf,
    },

    /// Service Account 认证
    ServiceAccount {
        provider: SAProvider,
        credentials_json: String,
    },
}

/// API Key 配置（统一结构）
///
/// 设计说明：
/// - API Key 本质上只是一个字符串，不区分官方/转发站
/// - 区分的关键是 `base_url`，这只是配置参数
/// - 真正决定请求格式的是 `provider` 字段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyConfig {
    /// API Key 字符串
    pub key: String,

    /// 自定义 Base URL
    /// - None: 使用官方端点
    /// - Some(url): 使用转发站/代理
    pub base_url: Option<String>,

    /// Provider 标识（用于选择请求格式）
    pub provider: ApiKeyProvider,
}

impl ApiKeyConfig {
    /// 创建新的 API Key 配置
    pub fn new(key: impl Into<String>, provider: ApiKeyProvider) -> Self {
        Self {
            key: key.into(),
            base_url: None,
            provider,
        }
    }

    /// 设置自定义 Base URL
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// 是否为官方端点
    pub fn is_official(&self) -> bool {
        self.base_url.is_none()
    }

    /// 获取 Base URL（官方或自定义）
    pub fn base_url_or<'a>(&self, default: &'a str) -> Cow<'a, str> {
        match &self.base_url {
            Some(url) => Cow::Owned(url.clone()),
            None => Cow::Borrowed(default),
        }
    }
}

/// API Key Provider 标识
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApiKeyProvider {
    Anthropic,
    OpenAI,
    GeminiAIStudio,
    Codex,
    IFlow,
}

impl ApiKeyProvider {
    /// 获取默认 Base URL
    pub fn default_base_url(&self) -> &'static str {
        match self {
            ApiKeyProvider::Anthropic => "https://api.anthropic.com",
            ApiKeyProvider::OpenAI => "https://api.openai.com",
            ApiKeyProvider::GeminiAIStudio => "https://generativelanguage.googleapis.com",
            ApiKeyProvider::Codex => "https://chatgpt.com/backend-api/codex",
            ApiKeyProvider::IFlow => "https://apis.iflow.cn",
        }
    }
}

/// OAuth Provider 标识
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OAuthProvider {
    Claude,
    GeminiCli,
    Antigravity,
}

/// Service Account Provider 标识
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SAProvider {
    VertexAI,
}

/// Token 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenStatus {
    /// 有效
    Valid,
    /// 即将过期
    ExpiringSoon,
    /// 已过期
    Expired,
    /// 刷新失败
    RefreshFailed,
}

/// 通用 Token 存储格式
///
/// 设计说明：
/// - 提供统一的存储格式，便于持久化
/// - 使用 `extra` 字段存储 Provider 特有数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenStorage {
    /// Access Token
    pub access_token: String,
    /// Refresh Token
    pub refresh_token: Option<String>,
    /// 过期时间
    pub expires_at: Option<DateTime<Utc>>,
    /// 用户邮箱
    pub email: Option<String>,
    /// Provider 类型标识
    pub provider: String,
    /// Provider 特有字段
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl TokenStorage {
    /// 创建新的 Token 存储
    pub fn new(access_token: impl Into<String>, provider: impl Into<String>) -> Self {
        Self {
            access_token: access_token.into(),
            refresh_token: None,
            expires_at: None,
            email: None,
            provider: provider.into(),
            extra: HashMap::new(),
        }
    }

    /// 设置 Refresh Token
    pub fn with_refresh_token(mut self, token: impl Into<String>) -> Self {
        self.refresh_token = Some(token.into());
        self
    }

    /// 设置过期时间
    pub fn with_expires_at(mut self, time: DateTime<Utc>) -> Self {
        self.expires_at = Some(time);
        self
    }

    /// 设置邮箱
    pub fn with_email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }

    /// 添加额外字段
    pub fn with_extra(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.extra.insert(key.into(), value);
        self
    }

    /// 检查 Token 状态
    pub fn status(&self, lead_seconds: i64) -> TokenStatus {
        let Some(expires_at) = self.expires_at else {
            return TokenStatus::Valid;
        };

        let now = Utc::now();
        let threshold = expires_at - chrono::Duration::seconds(lead_seconds);

        if now >= expires_at {
            TokenStatus::Expired
        } else if now >= threshold {
            TokenStatus::ExpiringSoon
        } else {
            TokenStatus::Valid
        }
    }

    /// 是否需要刷新
    pub fn needs_refresh(&self, lead_seconds: i64) -> bool {
        matches!(
            self.status(lead_seconds),
            TokenStatus::Expired | TokenStatus::ExpiringSoon
        )
    }
}

/// 认证错误
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Token 已过期")]
    TokenExpired,

    #[error("Token 刷新失败: {0}")]
    RefreshFailed(String),

    #[error("OAuth 认证失败: {0}")]
    OAuthFailed(String),

    #[error("无效的凭据: {0}")]
    InvalidCredentials(String),

    #[error("存储失败: {0}")]
    StorageError(String),

    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON 解析错误: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP 请求失败: {0}")]
    Http(String),
}

impl From<reqwest::Error> for AuthError {
    fn from(e: reqwest::Error) -> Self {
        AuthError::Http(e.to_string())
    }
}
