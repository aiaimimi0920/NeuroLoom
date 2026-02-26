use crate::auth::Authenticator;
use crate::site::context::AuthType;
use reqwest::RequestBuilder;

/// Anthropic API Key 认证
/// 特殊：使用 x-api-key header（而非标准的 Authorization: Bearer）
pub struct AnthropicApiKeyAuth {
    api_key: String,
}

impl AnthropicApiKeyAuth {
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            api_key: key.into(),
        }
    }
}

#[async_trait::async_trait]
impl Authenticator for AnthropicApiKeyAuth {
    fn id(&self) -> &str {
        "anthropic_api_key"
    }

    fn is_authenticated(&self) -> bool {
        !self.api_key.is_empty()
    }

    fn inject(&self, req: RequestBuilder) -> anyhow::Result<RequestBuilder> {
        // Anthropic 使用 x-api-key（不是 Authorization: Bearer）
        Ok(req.header("x-api-key", &self.api_key))
    }

    fn auth_type(&self) -> AuthType {
        AuthType::ApiKey
    }
}
