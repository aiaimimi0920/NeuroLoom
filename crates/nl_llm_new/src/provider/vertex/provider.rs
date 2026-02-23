//! Vertex AI (Gemini on GCP) Provider 实现
//!
//! 专用于 Google Cloud Vertex AI，通过 Service Account JSON 认证
//!
//! URL 格式: `https://{region}-aiplatform.googleapis.com/v1/projects/{proj}/locations/{region}/publishers/google/models/{model}:{action}`
//! 认证方式: RS256 JWT → Bearer token (Authorization header)

use super::config::VertexConfig;
use crate::auth::{Auth, SAProvider};
// Unused import removed
use tokio::sync::RwLock;
use std::sync::Arc;
use crate::provider::{Endpoint, GenericClient};
use crate::provider::gemini::provider::GeminiProtocol;
use crate::generic_client;
use async_trait::async_trait;
use std::time::{SystemTime, UNIX_EPOCH};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};

// ── 常量 ────────────────────────────────────────────────────────────────────────
const VERTEX_API_VERSION: &str = "v1";
const VERTEX_TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const VERTEX_CLOUD_PLATFORM_SCOPE: &str = "https://www.googleapis.com/auth/cloud-platform";
const VERTEX_DEFAULT_LOCATION: &str = "us-central1";

// ── 数据结构 ────────────────────────────────────────────────────────────────────

/// 从 SA JSON 解析出的核心字段
#[derive(Debug, Deserialize)]
struct ServiceAccount {
    project_id: String,
    client_email: String,
    private_key: String,
    #[allow(dead_code)]
    #[serde(default)]
    private_key_id: String,
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
    #[allow(dead_code)]
    expires_in: u64,
    #[allow(dead_code)]
    token_type: String,
}


// ── 凭证与状态管理 ─────────────────────────────────────────────────────────────

/// SA 认证状态缓存
#[derive(Debug, Default)]
struct AuthState {
    access_token: String,
    expires_at: u64,
}

impl AuthState {
    fn is_valid(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        // 提前 5 分钟刷新
        self.expires_at > now + 300
    }
}

pub struct VertexEndpoint {
    config: VertexConfig,
    auth_state: Arc<RwLock<AuthState>>,
    http: reqwest::Client,
}

impl VertexEndpoint {
    async fn refresh_auth_internal(&self) -> crate::Result<()> {
        let (token, expires_in) = self.exchange_jwt_for_token(&self.config.credentials_json).await?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let mut state = self.auth_state.write().await;
        state.access_token = token;
        state.expires_at = now + expires_in;
        Ok(())
    }

    /// 解析 SA JSON，签发 JWT，向 Google token 端点换取 access_token
    async fn exchange_jwt_for_token(&self, sa_json: &str) -> crate::Result<(String, u64)> {
        // 1. 解析服务账号 JSON
        let sa: ServiceAccount = serde_json::from_str(sa_json).map_err(|e| {
            crate::Error::Provider(crate::provider::ProviderError::fail(format!(
                "vertex: failed to parse service account JSON: {}",
                e
            )))
        })?;

        // 2. 规范化私钥
        let pem = normalize_private_key(&sa.private_key).map_err(|e| {
            crate::Error::Provider(crate::provider::ProviderError::fail(format!(
                "vertex: invalid private key: {}",
                e
            )))
        })?;

        // 3. 构建 JWT Claims
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

        // 4. 用 RSA 私钥（RS256）签发 JWT
        let encoding_key = EncodingKey::from_rsa_pem(pem.as_bytes()).map_err(|e| {
            crate::Error::Provider(crate::provider::ProviderError::fail(format!(
                "vertex: failed to load RSA private key: {}",
                e
            )))
        })?;
        let jwt = encode(&Header::new(Algorithm::RS256), &claims, &encoding_key).map_err(|e| {
            crate::Error::Provider(crate::provider::ProviderError::fail(format!(
                "vertex: failed to sign JWT: {}",
                e
            )))
        })?;

        // 5. POST 换取 access_token
        let grant_type = "urn:ietf:params:oauth:grant-type:jwt-bearer";
        let params = [("grant_type", grant_type), ("assertion", jwt.as_str())];
        let resp = self
            .http
            .post(VERTEX_TOKEN_ENDPOINT)
            .form(&params)
            .send()
            .await
            .map_err(|e| crate::Error::Http(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(crate::Error::Provider(crate::provider::ProviderError::fail(
                format!("vertex: token exchange failed ({}): {}", status, body.trim()),
            )));
        }

        let token_resp: GoogleTokenResponse = resp.json().await.map_err(|e| {
            crate::Error::Provider(crate::provider::ProviderError::fail(format!(
                "vertex: failed to parse token response: {}",
                e
            )))
        })?;

        Ok((token_resp.access_token, token_resp.expires_in))
    }

    /// 从 SA JSON 中提取 project_id
    fn project_id(&self) -> crate::Result<String> {
        let sa: ServiceAccount = serde_json::from_str(&self.config.credentials_json).map_err(
            |e| {
                crate::Error::Provider(crate::provider::ProviderError::fail(format!(
                    "vertex: failed to parse service account for project_id: {}",
                    e
                )))
            },
        )?;
        Ok(sa.project_id)
    }

    fn location(&self) -> &str {
        self.config
            .location
            .as_deref()
            .unwrap_or(VERTEX_DEFAULT_LOCATION)
    }
}

#[async_trait]
impl Endpoint for VertexEndpoint {
    async fn pre_flight(&self) -> crate::Result<()> {
        let state = self.auth_state.read().await;
        if !state.is_valid() {
            drop(state);
            self.refresh_auth_internal().await?;
        }
        Ok(())
    }

    fn url(&self, model: &str, is_stream: bool) -> crate::Result<String> {
        let action = if is_stream { "streamGenerateContent?alt=sse" } else { "generateContent" };
        let project_id = self.project_id()?;
        let location = self.location();
        let default_base = vertex_base_url(location);
        let base = self.config.base_url.as_deref().unwrap_or(&default_base);
        
        Ok(format!(
            "{}/{}/projects/{}/locations/{}/publishers/google/models/{}:{}",
            base, VERTEX_API_VERSION, project_id, location, model, action
        ))
    }

    fn inject_auth(&self, req: reqwest::RequestBuilder) -> crate::Result<reqwest::RequestBuilder> {
        // 由于预检过程(pre_flight)已确保 valid，这里安全 block 并提取
        // 为了避免复杂的 async 嵌套（inject_auth 是同步签名），我们可以用 try_read 或者直接这里假定 pre_flight 做了保证。
        // 由于 get_access_token() 是 async 的，但我们这里是同步的，所以我们读取当前状态即可
        let token = {
            let Ok(state) = self.auth_state.try_read() else {
                return Err(crate::Error::Provider(crate::provider::ProviderError::fail("vertex: auth state locked")));
            };
            if !state.is_valid() {
                return Err(crate::Error::Provider(crate::provider::ProviderError::fail("vertex: auth state invalid during inject_auth")));
            }
            state.access_token.clone()
        };

        Ok(req.header("Authorization", format!("Bearer {}", token)))
    }

    fn needs_refresh(&self) -> bool {
        let Ok(state) = self.auth_state.try_read() else {
            return true;
        };
        !state.is_valid()
    }

    async fn refresh_auth(&self) -> crate::Result<()> {
        self.refresh_auth_internal().await
    }
}

pub type VertexProvider = GenericClient<VertexEndpoint, GeminiProtocol>;

impl VertexProvider {
    /// 创建新的 Vertex Provider（需要外部传入 HTTP Client）
    ///
    /// 注意：根据设计规范，HTTP Client 应由外部统一管理，
    /// 避免每个 Provider 重复创建连接池。
    pub fn new(config: VertexConfig, http: reqwest::Client) -> Self {
        let auth_enum = Auth::ServiceAccount {
            provider: SAProvider::VertexAI,
            credentials_json: config.credentials_json.clone(),
        };

        let endpoint = VertexEndpoint {
            config,
            auth_state: Arc::new(RwLock::new(AuthState::default())),
            http: http.clone(),
        };

        generic_client! {
            id: "vertex".to_string(),
            endpoint: endpoint,
            protocol: GeminiProtocol,
            auth: auth_enum,
            supported_models: vec![
                "gemini-1.5-pro".to_string(),
                "gemini-1.5-flash".to_string(),
                "gemini-2.0-flash".to_string(),
                "gemini-2.0-pro-exp-02-05".to_string(),
                "gemini-2.5-flash".to_string(),
                "gemini-2.5-pro".to_string(),
            ],
            http: http
        }
    }

    /// 以 SA JSON 字符串构建，指定模型和区域
    pub fn from_service_account(
        credentials_json: impl Into<String>,
        model: impl Into<String>,
        location: Option<String>,
        http: reqwest::Client,
    ) -> Self {
        Self::new(VertexConfig {
            credentials_json: credentials_json.into(),
            location,
            model: model.into(),
            base_url: None,
        }, http)
    }

    /// 从 SA JSON 文件加载
    pub fn from_file(
        path: &std::path::Path,
        model: impl Into<String>,
        location: Option<String>,
        http: reqwest::Client,
    ) -> crate::Result<Self> {
        let credentials_json = std::fs::read_to_string(path)?;
        Ok(Self::from_service_account(credentials_json, model, location, http))
    }
}


// ── 独立辅助函数 ─────────────────────────────────────────────────────────────────

/// 返回 Vertex AI 的区域 base URL
pub fn vertex_base_url(location: &str) -> String {
    let loc = location.trim();
    if loc.is_empty() {
        return format!(
            "https://{}-aiplatform.googleapis.com",
            VERTEX_DEFAULT_LOCATION
        );
    }
    if loc == "global" {
        return "https://aiplatform.googleapis.com".to_string();
    }
    format!("https://{}-aiplatform.googleapis.com", loc)
}

/// 规范化 PEM 私钥：接受 PKCS#1 和 PKCS#8 格式，统一为可被 jsonwebtoken 接受的 PEM
pub fn normalize_private_key(raw: &str) -> Result<String, String> {
    // 步骤 1：如果含有字面 \n（两字符 backslash+n），先转换为实际换行
    let pk = if raw.contains("\\n") {
        raw.replace("\\n", "\n")
    } else {
        raw.to_string()
    };

    // 步骤 2：规范化 CRLF 和孤立 CR
    let pk = pk.replace("\r\n", "\n").replace('\r', "\n");
    let pk = pk.trim().to_string();

    // 步骤 3：验证 PEM markers
    if pk.contains("-----BEGIN RSA PRIVATE KEY-----") || pk.contains("-----BEGIN PRIVATE KEY-----") {
        return Ok(pk);
    }

    // 安全截取前 80 字节（避免 UTF-8 边界 panic）
    let preview_len = raw
        .char_indices()
        .take(80)
        .last()
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(raw.len());
    Err(format!(
        "private_key does not contain PEM markers. First 80 chars: {}",
        &raw[..preview_len]
    ))
}

// ── 测试 ─────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertex_base_url() {
        assert_eq!(
            vertex_base_url("us-central1"),
            "https://us-central1-aiplatform.googleapis.com"
        );
        assert_eq!(vertex_base_url("global"), "https://aiplatform.googleapis.com");
        assert_eq!(
            vertex_base_url(""),
            "https://us-central1-aiplatform.googleapis.com"
        );
        assert_eq!(
            vertex_base_url("asia-northeast1"),
            "https://asia-northeast1-aiplatform.googleapis.com"
        );
    }

    #[test]
    fn test_normalize_private_key_literal_newline() {
        let pk = "-----BEGIN RSA PRIVATE KEY-----\\nMIIEpAIBAAKCAQEA\\n-----END RSA PRIVATE KEY-----";
        let result = normalize_private_key(pk).unwrap();
        assert!(result.contains("-----BEGIN RSA PRIVATE KEY-----"));
        assert!(result.contains('\n'));
    }

    #[test]
    fn test_normalize_private_key_actual_newline() {
        let pk = "-----BEGIN PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0BAQEFAASC\n-----END PRIVATE KEY-----";
        let result = normalize_private_key(pk).unwrap();
        assert!(result.contains("-----BEGIN PRIVATE KEY-----"));
    }

    #[test]
    fn test_normalize_private_key_invalid() {
        let result = normalize_private_key("not-a-pem-key");
        assert!(result.is_err());
    }

    #[test]
    fn test_from_service_account() {
        let sa_json = r#"{"project_id":"test-proj","client_email":"test@test.iam.gserviceaccount.com","private_key":"-----BEGIN PRIVATE KEY-----\ntest\n-----END PRIVATE KEY-----","private_key_id":""}"#;
        let http = reqwest::Client::new();
        let provider = VertexProvider::from_service_account(
            sa_json.to_string(),
            "gemini-2.5-flash".to_string(),
            Some("us-west1".to_string()),
            http,
        );
        assert_eq!(provider.endpoint.config.model, "gemini-2.5-flash");
        assert_eq!(provider.endpoint.config.location, Some("us-west1".to_string()));
    }

    #[test]
    fn test_build_url() {
        let config = VertexConfig {
            credentials_json: r#"{"project_id":"my-proj","client_email":"e@x.iam.gserviceaccount.com","private_key":"","private_key_id":""}"#.to_string(),
            location: Some("us-central1".to_string()),
            model: "gemini-2.5-flash".to_string(),
            base_url: None,
        };
        let http = reqwest::Client::new();
        let provider = VertexProvider::new(config, http);

        let url = provider.endpoint.url("gemini-2.5-flash", false).unwrap();
        assert_eq!(
            url,
            "https://us-central1-aiplatform.googleapis.com/v1/projects/my-proj/locations/us-central1/publishers/google/models/gemini-2.5-flash:generateContent"
        );
    }

    #[test]
    fn test_build_url_custom_base() {
        let config = VertexConfig {
            credentials_json: r#"{"project_id":"my-proj","client_email":"e@x.iam.gserviceaccount.com","private_key":"","private_key_id":""}"#.to_string(),
            location: Some("us-central1".to_string()),
            model: "gemini-2.5-flash".to_string(),
            base_url: Some("https://custom.vertex.api".to_string()),
        };
        let http = reqwest::Client::new();
        let provider = VertexProvider::new(config, http);

        let url = provider.endpoint.url("gemini-2.5-flash", false).unwrap();
        assert_eq!(
            url,
            "https://custom.vertex.api/v1/projects/my-proj/locations/us-central1/publishers/google/models/gemini-2.5-flash:generateContent"
        );
    }
}
