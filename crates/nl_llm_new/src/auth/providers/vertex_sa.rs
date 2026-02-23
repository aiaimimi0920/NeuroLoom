use crate::auth::{AuthError, TokenStatus, TokenStorage};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const VERTEX_TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const VERTEX_CLOUD_PLATFORM_SCOPE: &str = "https://www.googleapis.com/auth/cloud-platform";

/// 从 SA JSON 解析出的核心字段
#[derive(Debug, Deserialize)]
struct ServiceAccount {
    pub project_id: String,
    pub client_email: String,
    pub private_key: String,
}

/// JWT Claims（用于向 Google token 端点换取 Bearer token）
#[derive(Debug, Serialize)]
struct JwtClaims {
    iss: String,
    sub: String,
    aud: String,
    iat: u64,
    exp: u64,
    scope: String,
}

/// Google token 端点响应
#[derive(Debug, Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
}

/// Vertex AI Service Account 认证
///
/// 负责 Service Account JSON 到 Access Token 的免密兑换和持久化缓存
pub struct VertexSAAuth {
    /// 绑定的缓存文件路径
    path: Option<PathBuf>,
    /// 当前 Token（封装了 Access Token 和过期时间）
    pub token: Option<TokenStorage>,
    /// SA 的 JSON 原文
    credentials_json: String,
    /// HTTP Client
    http: reqwest::Client,
}

impl VertexSAAuth {
    pub fn new(credentials_json: impl Into<String>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client for VertexSAAuth");

        Self {
            path: None,
            token: None,
            credentials_json: credentials_json.into(),
            http,
        }
    }

    pub fn from_file(path: &Path) -> Result<Self, AuthError> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client for VertexSAAuth");

        // 如果路径是 .json，说明它是 SA 文件本身而不是用来存 Token 的缓存
        if path.extension().is_some_and(|ext| ext == "json") {
            let content = std::fs::read_to_string(path).map_err(|e| {
                AuthError::InvalidCredentials(format!("Failed to read SA JSON file: {}", e))
            })?;
            return Ok(Self {
                path: None, // SA 原文文件不充当 Token Cache 路径
                token: None,
                credentials_json: content,
                http,
            });
        }

        // 默认按 TokenStorage 兜底恢复
        let content = std::fs::read_to_string(path).unwrap_or_default();
        match serde_json::from_str::<TokenStorage>(&content) {
            Ok(token) => Ok(Self {
                path: Some(path.to_path_buf()),
                credentials_json: token.extra.get("sa_json").and_then(|v| v.as_str()).unwrap_or("{}").to_string(),
                token: Some(token),
                http,
            }),
            Err(e) => {
                eprintln!("Warning: Failed to parse token file: {}. Expect SA JSON to be bound later.", e);
                Ok(Self {
                    path: Some(path.to_path_buf()),
                    token: None,
                    credentials_json: "{}".to_string(),
                    http,
                })
            }
        }
    }
    
    /// 当外部指定了 SA JSON 时，覆盖缓存内潜在绑定的内容
    pub fn bind_credentials(&mut self, json_str: String) {
        self.credentials_json = json_str;
    }
    
    /// 提取 GCP Project ID 
    pub fn project_id(&self) -> Result<String, AuthError> {
        let sa: ServiceAccount = serde_json::from_str(&self.credentials_json).map_err(|e| {
            AuthError::InvalidCredentials(format!("failed to parse service account JSON: {}", e))
        })?;
        Ok(sa.project_id)
    }

    /// 获取 Token 状态 (Vertex 属于短命 Token)
    pub fn token_status(&self) -> TokenStatus {
        self.token.as_ref().map_or(TokenStatus::Expired, |t| {
            if t.access_token.is_empty() {
                return TokenStatus::Expired;
            }
            // 使用 5 分钟 (300秒) 的提前量检查过期（Vertex 通常存活1小时）
            t.status(300)
        })
    }

    /// 检查是否需要刷新
    pub fn needs_refresh(&self) -> bool {
        matches!(self.token_status(), TokenStatus::Expired | TokenStatus::ExpiringSoon)
    }

    /// 确保安全获取到有效 Token（内部执行刷新）
    pub async fn ensure_authenticated(&mut self) -> Result<(), AuthError> {
        if self.credentials_json.is_empty() || self.credentials_json == "{}" {
            return Err(AuthError::InvalidCredentials("No valid Service Account JSON provided".to_string()));
        }

        if self.needs_refresh() {
            let api_key = self.exchange_jwt_for_token_static().await?;

            if let Some(ref mut token) = self.token {
                token.access_token = api_key.clone();
                token.expires_at = Some(chrono::Utc::now() + chrono::Duration::hours(1));
                
                if let Some(ref path) = self.path {
                    let _ = Self::save_token_to_path_static(token, path);
                }
            } else {
                let mut extra = std::collections::HashMap::new();
                extra.insert("sa_json".to_string(), serde_json::Value::String(self.credentials_json.clone()));
                
                let new_token = TokenStorage {
                    access_token: api_key,
                    refresh_token: None,
                    expires_at: Some(chrono::Utc::now() + chrono::Duration::hours(1)),
                    email: None,
                    provider: "VertexSA".to_string(),
                    extra,
                };
                
                if let Some(ref path) = self.path {
                    let _ = Self::save_token_to_path_static(&new_token, path);
                }
                
                self.token = Some(new_token);
            }
        }
        Ok(())
    }

    /// 暴露当前立即可用的 Access Token
    pub fn token(&self) -> Option<&str> {
        self.token.as_ref().map(|t| t.access_token.as_str()).filter(|s| !s.is_empty())
    }

    /// 执行 RSA 签名到 OAuth 端点的主动交换
    async fn exchange_jwt_for_token_static(&self) -> Result<String, AuthError> {
        let sa: ServiceAccount = serde_json::from_str(&self.credentials_json).map_err(|e| {
            AuthError::InvalidCredentials(format!("failed to parse service account JSON: {}", e))
        })?;

        let pem = Self::normalize_private_key(&sa.private_key).map_err(|e| {
            AuthError::InvalidCredentials(format!("invalid private key: {}", e))
        })?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        let claims = JwtClaims {
            iss: sa.client_email.clone(),
            sub: sa.client_email.clone(),
            aud: VERTEX_TOKEN_ENDPOINT.to_string(),
            iat: now,
            exp: now + 3600,
            scope: VERTEX_CLOUD_PLATFORM_SCOPE.to_string(),
        };

        let encoding_key = EncodingKey::from_rsa_pem(pem.as_bytes()).map_err(|e| {
            AuthError::InvalidCredentials(format!("failed to load RSA private key: {}", e))
        })?;
        
        let jwt = encode(&Header::new(Algorithm::RS256), &claims, &encoding_key).map_err(|e| {
            AuthError::RefreshFailed(format!("failed to sign JWT: {}", e))
        })?;

        let params = [
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", jwt.as_str())
        ];
        
        let resp = self
            .http
            .post(VERTEX_TOKEN_ENDPOINT)
            .form(&params)
            .send()
            .await
            .map_err(|e| AuthError::Http(format!("vertex token exchange failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AuthError::Http(format!("token exchange replied ({}) failed: {}", status, body.trim())));
        }

        let token_resp: GoogleTokenResponse = resp.json().await.map_err(|e| {
            AuthError::Http(format!("failed to decode token json response: {}", e))
        })?;

        Ok(token_resp.access_token)
    }

    /// 保存 Token 到文件
    fn save_token_to_path_static(
        token: &TokenStorage,
        path: &Path,
    ) -> Result<(), std::io::Error> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(token)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// 规范化 PEM 私钥 (兼容 \n 以及 \r\n 乱码)
    fn normalize_private_key(raw: &str) -> Result<String, String> {
        let pk = if raw.contains("\\n") {
            raw.replace("\\n", "\n")
        } else {
            raw.to_string()
        };
        let pk = pk.replace("\r\n", "\n").replace('\r', "\n");
        let pk = pk.trim().to_string();

        if pk.contains("-----BEGIN RSA PRIVATE KEY-----") || pk.contains("-----BEGIN PRIVATE KEY-----") {
            return Ok(pk);
        }

        let preview_len = raw
            .char_indices()
            .take(50)
            .last()
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(raw.len());
        Err(format!("private_key does not contain PEM markers. First 50 chars: {}", &raw[..preview_len]))
    }
}
