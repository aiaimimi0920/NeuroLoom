use reqwest::RequestBuilder;

use crate::auth::Authenticator;
use crate::site::context::AuthType;

/// Vidu API Key 认证
///
/// 特殊：使用 `Authorization: Token <api_key>`（不是 Bearer）
pub struct ViduApiKeyAuth {
    api_key: String,
}

impl ViduApiKeyAuth {
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            api_key: key.into(),
        }
    }
}

#[async_trait::async_trait]
impl Authenticator for ViduApiKeyAuth {
    fn id(&self) -> &str {
        "vidu_api_key"
    }

    fn is_authenticated(&self) -> bool {
        !self.api_key.is_empty()
    }

    fn inject(&self, req: RequestBuilder) -> anyhow::Result<RequestBuilder> {
        Ok(req.header("Authorization", format!("Token {}", self.api_key)))
    }

    fn auth_type(&self) -> AuthType {
        AuthType::ApiKey
    }
}
