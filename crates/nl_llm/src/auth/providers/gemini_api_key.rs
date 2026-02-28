use async_trait::async_trait;
use reqwest::RequestBuilder;

use crate::auth::Authenticator;
use crate::site::context::AuthType;

/// Gemini 官方 API Key 认证器
///
/// 与通用的 `ApiKeyAuth` 不同，Gemini 的 API Key 通过 URL query 参数 (`?key=xxx`)
/// 传递，由 `GeminiSite::build_url()` 负责拼接。此认证器的 `inject()` 不注入 Header。
pub struct GeminiApiKeyAuth {
    key: String,
}

impl GeminiApiKeyAuth {
    pub fn new(key: impl Into<String>) -> Self {
        Self { key: key.into() }
    }

    /// 获取存储的 API Key（供 ClientBuilder 透传到 GeminiSite）
    pub fn key(&self) -> &str {
        &self.key
    }
}

#[async_trait]
impl Authenticator for GeminiApiKeyAuth {
    fn id(&self) -> &str {
        "gemini_api_key"
    }

    fn is_authenticated(&self) -> bool {
        !self.key.is_empty()
    }

    fn inject(&self, req: RequestBuilder) -> anyhow::Result<RequestBuilder> {
        // Gemini API Key 在 URL query 中注入，不需要 Header
        Ok(req)
    }

    fn auth_type(&self) -> AuthType {
        AuthType::ApiKey
    }
}
