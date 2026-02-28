use crate::site::context::AuthType;
use crate::auth::traits::Authenticator;
use async_trait::async_trait;
use reqwest::RequestBuilder;

/// 百川智能 API Key 认证
#[derive(Debug, Clone)]
pub struct BaichuanAuth {
    pub api_key: String,
}

impl BaichuanAuth {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
        }
    }
}

#[async_trait]
impl Authenticator for BaichuanAuth {
    fn id(&self) -> &str {
        "baichuan"
    }

    fn is_authenticated(&self) -> bool {
        !self.api_key.is_empty()
    }

    fn needs_refresh(&self) -> bool {
        false // API Key 不会动态过期
    }

    async fn refresh(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn inject(&self, req: RequestBuilder) -> anyhow::Result<RequestBuilder> {
        // 百川使用标准的 Authorization: Bearer <API_KEY> 头，符合通用 OpenAI 规范
        Ok(req.header("Authorization", format!("Bearer {}", self.api_key)))
    }

    fn auth_type(&self) -> AuthType {
        AuthType::ApiKey
    }
}
