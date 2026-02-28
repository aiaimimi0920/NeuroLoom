use async_trait::async_trait;
use reqwest::RequestBuilder;

use crate::auth::Authenticator;
use crate::site::context::AuthType;

/// API Key 认证器
pub struct ApiKeyAuth {
    pub key: String,
    pub header_name: String,
    pub is_bearer: bool,
}

impl ApiKeyAuth {
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            header_name: "Authorization".into(),
            is_bearer: true,
        }
    }

    pub fn with_header(mut self, header_name: impl Into<String>, is_bearer: bool) -> Self {
        self.header_name = header_name.into();
        self.is_bearer = is_bearer;
        self
    }
}

#[async_trait]
impl Authenticator for ApiKeyAuth {
    fn id(&self) -> &str {
        "api_key"
    }

    fn is_authenticated(&self) -> bool {
        !self.key.is_empty()
    }

    fn inject(&self, req: RequestBuilder) -> anyhow::Result<RequestBuilder> {
        let val = if self.is_bearer {
            format!("Bearer {}", self.key)
        } else {
            self.key.clone()
        };
        Ok(req.header(&self.header_name, val))
    }

    fn auth_type(&self) -> AuthType {
        AuthType::ApiKey
    }
}
