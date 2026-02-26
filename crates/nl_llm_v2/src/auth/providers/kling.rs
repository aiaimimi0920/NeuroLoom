use async_trait::async_trait;
use jsonwebtoken::{encode, EncodingKey, Header};
use reqwest::RequestBuilder;
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::auth::Authenticator;
use crate::site::context::AuthType;

/// 可灵 AI 认证器 (Kling)
///
/// 使用 AccessKey 和 SecretKey 签发 HS256 JWT。
/// 凭证格式要求为: `AccessKey|SecretKey`
pub struct KlingAuth {
    access_key: String,
    secret_key: String,
}

#[derive(Debug, Serialize)]
struct KlingJwtClaims {
    iss: String,
    exp: u64,
    nbf: u64,
}

impl KlingAuth {
    pub fn new(credentials: impl Into<String>) -> Self {
        let creds = credentials.into();
        
        let access_key;
        let mut secret_key = String::new();

        let parts: Vec<&str> = creds.split('|').collect();
        if parts.len() == 2 {
            access_key = parts[0].trim().to_string();
            secret_key = parts[1].trim().to_string();
        } else {
            // 如果只有一段，降级为普通的 token（适配某些新 API 代理场景）
            access_key = creds.clone();
        }

        Self {
            access_key,
            secret_key,
        }
    }

    /// 生成 HS256 JWT Token
    fn create_jwt_token(&self) -> anyhow::Result<String> {
        if self.secret_key.is_empty() {
            // 没有 Secret Key 时，直接返回 Access Key 作为 Token（代理站兼容）
            return Ok(self.access_key.clone());
        }

        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let claims = KlingJwtClaims {
            iss: self.access_key.clone(),
            exp: now + 1800, // 30 分钟过期
            nbf: now.saturating_sub(5),
        };

        let mut header = Header::default();
        header.typ = Some("JWT".to_string());
        // jsonwebtoken 的 default 就是 HS256

        let token = encode(
            &header,
            &claims,
            &EncodingKey::from_secret(self.secret_key.as_bytes()),
        )?;

        Ok(token)
    }
}

#[async_trait]
impl Authenticator for KlingAuth {
    fn id(&self) -> &str {
        "kling"
    }

    fn is_authenticated(&self) -> bool {
        !self.access_key.is_empty()
    }

    fn inject(&self, req: RequestBuilder) -> anyhow::Result<RequestBuilder> {
        let token = self.create_jwt_token()?;
        Ok(req.header("Authorization", format!("Bearer {}", token)))
    }

    fn auth_type(&self) -> AuthType {
        // Jwt 每次动态生成并作为 Bearer Token 注入，可归类为普通的 API Key 类型，因为不需要控制 URL
        AuthType::ApiKey
    }
}
