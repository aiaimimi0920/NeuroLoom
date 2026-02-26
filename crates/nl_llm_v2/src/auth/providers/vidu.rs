use async_trait::async_trait;
use reqwest::RequestBuilder;

use crate::auth::Authenticator;
use crate::site::context::AuthType;

/// Vidu 官方 API 认证器
///
/// Vidu 使用：
/// - `Authorization: Token <api_key>`
///
/// 该实现只负责注入 Header。
pub struct ViduAuth {
    api_key: String,
}

impl ViduAuth {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
        }
    }
}

#[async_trait]
impl Authenticator for ViduAuth {
    fn id(&self) -> &str {
        "vidu"
    }

    fn is_authenticated(&self) -> bool {
        !self.api_key.trim().is_empty()
    }

    fn inject(&self, req: RequestBuilder) -> anyhow::Result<RequestBuilder> {
        Ok(req.header("Authorization", format!("Token {}", self.api_key)))
    }

    fn auth_type(&self) -> AuthType {
        AuthType::ApiKey
    }
}
