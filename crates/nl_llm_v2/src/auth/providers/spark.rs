use async_trait::async_trait;
use reqwest::RequestBuilder;

use crate::auth::Authenticator;
use crate::site::context::AuthType;

/// 讯飞星火认证器。
///
/// 支持两种输入：
/// - APIPassword（官方 HTTP OpenAPI 推荐）
/// - `APIKey:APISecret`（兼容部分历史接入方式）
///
/// 注入格式统一为 `Authorization: Bearer <token>`。
pub struct SparkAuth {
    token: String,
}

impl SparkAuth {
    pub fn new(token: impl Into<String>) -> Self {
        let token = token
            .into()
            .trim()
            .trim_start_matches("Bearer ")
            .trim_start_matches("bearer ")
            .to_string();
        Self { token }
    }
}

#[async_trait]
impl Authenticator for SparkAuth {
    fn id(&self) -> &str {
        "spark"
    }

    fn is_authenticated(&self) -> bool {
        !self.token.is_empty()
    }

    fn inject(&self, req: RequestBuilder) -> anyhow::Result<RequestBuilder> {
        Ok(req.header("Authorization", format!("Bearer {}", self.token)))
    }

    fn auth_type(&self) -> AuthType {
        // 设计上星火属于多字段签名体系；即便传入 APIPassword，也统一标识为 MultiKey。
        AuthType::MultiKey
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn injects_bearer_header_with_normalized_token() {
        let auth = SparkAuth::new("Bearer sk-test");
        let client = reqwest::Client::new();
        let req = client.post("https://example.com");
        let req = auth
            .inject(req)
            .expect("inject should succeed")
            .build()
            .unwrap();
        assert_eq!(
            req.headers().get("Authorization").unwrap(),
            "Bearer sk-test"
        );
    }

    #[test]
    fn auth_type_is_multikey() {
        let auth = SparkAuth::new("api_key:api_secret");
        assert_eq!(auth.auth_type(), AuthType::MultiKey);
    }
}
