use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use async_trait::async_trait;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use reqwest::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};

use crate::auth::traits::Authenticator;
use crate::auth::types::{TokenStatus, TokenStorage};
use crate::site::context::AuthType;

const VERTEX_TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const VERTEX_CLOUD_PLATFORM_SCOPE: &str = "https://www.googleapis.com/auth/cloud-platform";

#[derive(Debug, Deserialize)]
struct ServiceAccount {
    pub client_email: String,
    pub private_key: String,
}

#[derive(Debug, Serialize)]
struct JwtClaims {
    iss: String,
    sub: String,
    aud: String,
    iat: u64,
    exp: u64,
    scope: String,
}

#[derive(Debug, Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
}

/// Google Service Account 认证器
pub struct ServiceAccountAuth {
    token: Option<TokenStorage>,
    credentials_json: String,
    http: Client,
    cache_path: Option<PathBuf>,
}

impl ServiceAccountAuth {
    pub fn new(credentials_json: impl Into<String>) -> Self {
        Self {
            token: None,
            credentials_json: credentials_json.into(),
            http: Client::new(),
            cache_path: None,
        }
    }

    pub fn with_cache(mut self, path: impl AsRef<Path>) -> Self {
        self.cache_path = Some(path.as_ref().to_path_buf());
        if let Some(p) = &self.cache_path {
            if p.exists() {
                if let Ok(content) = std::fs::read_to_string(p) {
                    if let Ok(token) = serde_json::from_str::<TokenStorage>(&content) {
                        self.token = Some(token);
                    }
                }
            }
        }
        self
    }
}

#[async_trait]
impl Authenticator for ServiceAccountAuth {
    fn id(&self) -> &str {
        "service_account"
    }

    fn is_authenticated(&self) -> bool {
        self.token.is_some()
    }

    fn needs_refresh(&self) -> bool {
        self.token.as_ref().map_or(true, |t| {
            matches!(t.status(300), TokenStatus::Expired | TokenStatus::ExpiringSoon)
        })
    }

    async fn refresh(&mut self) -> anyhow::Result<()> {
        let sa: ServiceAccount = serde_json::from_str(&self.credentials_json)
            .map_err(|e| anyhow::anyhow!("Invalid SA JSON: {}", e))?;

        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let claims = JwtClaims {
            iss: sa.client_email.clone(),
            sub: sa.client_email,
            aud: VERTEX_TOKEN_ENDPOINT.to_string(),
            iat: now,
            exp: now + 3600,
            scope: VERTEX_CLOUD_PLATFORM_SCOPE.to_string(),
        };

        let header = Header::new(Algorithm::RS256);
        let key = EncodingKey::from_rsa_pem(sa.private_key.as_bytes())?;
        let jwt = encode(&header, &claims, &key)?;

        let params = [
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", &jwt),
        ];

        let res = self.http.post(VERTEX_TOKEN_ENDPOINT)
            .form(&params)
            .send()
            .await?;

        if !res.status().is_success() {
            let body = res.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Token request failed: {}", body));
        }

        let token_resp: GoogleTokenResponse = res.json().await?;
        
        let token_info = TokenStorage {
            access_token: token_resp.access_token,
            refresh_token: None,
            expires_at: Some(chrono::Utc::now() + chrono::Duration::seconds(3600)),
            email: None,
            provider: "ServiceAccount".to_string(),
            extra: std::collections::HashMap::new(),
        };

        self.token = Some(token_info.clone());

        if let Some(path) = &self.cache_path {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(path, serde_json::to_string_pretty(&token_info)?);
        }

        Ok(())
    }

    fn inject(&self, req: RequestBuilder) -> anyhow::Result<RequestBuilder> {
        if let Some(t) = &self.token {
            Ok(req.bearer_auth(&t.access_token))
        } else {
            Err(anyhow::anyhow!("Not authenticated"))
        }
    }

    fn auth_type(&self) -> AuthType {
        AuthType::ServiceAccount
    }
}
