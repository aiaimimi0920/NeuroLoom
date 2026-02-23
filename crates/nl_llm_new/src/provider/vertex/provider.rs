//! Vertex AI (Gemini on GCP) Provider 实现
//!
//! 专用于 Google Cloud Vertex AI，通过 Service Account JSON 认证
//!
//! URL 格式: `https://{region}-aiplatform.googleapis.com/v1/projects/{proj}/locations/{region}/publishers/google/models/{model}:{action}`
//! 认证方式: RS256 JWT → Bearer token (Authorization header)

use super::config::VertexConfig;
use crate::auth::{Auth, SAProvider};
use crate::primitive::PrimitiveRequest;
use crate::provider::gemini::{compile_request, parse_response, parse_sse_stream};
use crate::provider::{BoxStream, LlmChunk, LlmProvider, LlmResponse};
use async_trait::async_trait;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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

// ── VertexProvider ───────────────────────────────────────────────────────────────

/// Vertex AI Provider
///
/// 通过 Service Account JSON 认证，调用 Vertex AI Gemini API
pub struct VertexProvider {
    config: VertexConfig,
    http: reqwest::Client,
    auth_enum: Auth,
}

impl VertexProvider {
    /// 创建新的 Vertex Provider
    pub fn new(config: VertexConfig) -> Self {
        Self::with_client(
            config,
            reqwest::Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .expect("Failed to create HTTP client"),
        )
    }

    /// 使用外部指定的 HTTP Client 创建 Provider
    pub fn with_client(config: VertexConfig, http: reqwest::Client) -> Self {
        let auth_enum = Auth::ServiceAccount {
            provider: SAProvider::VertexAI,
            credentials_json: config.credentials_json.clone(),
        };

        Self {
            config,
            http,
            auth_enum,
        }
    }

    /// 以 SA JSON 字符串构建，指定模型和区域
    pub fn from_service_account(
        credentials_json: impl Into<String>,
        model: impl Into<String>,
        location: Option<String>,
    ) -> Self {
        Self::new(VertexConfig {
            credentials_json: credentials_json.into(),
            location,
            model: model.into(),
            base_url: None,
        })
    }

    /// 从 SA JSON 文件加载
    pub fn from_file(
        path: &std::path::Path,
        model: impl Into<String>,
        location: Option<String>,
    ) -> crate::Result<Self> {
        let credentials_json = std::fs::read_to_string(path)?;
        Ok(Self::from_service_account(credentials_json, model, location))
    }

    // ── 认证 ─────────────────────────────────────────────────────────────────────

    /// 获取 Bearer access token（通过 SA JSON 签发 JWT 换取）
    async fn get_access_token(&self) -> crate::Result<String> {
        self.exchange_jwt_for_token(&self.config.credentials_json)
            .await
    }

    /// 解析 SA JSON，签发 JWT，向 Google token 端点换取 access_token
    async fn exchange_jwt_for_token(&self, sa_json: &str) -> crate::Result<String> {
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

        Ok(token_resp.access_token)
    }

    // ── 辅助方法 ─────────────────────────────────────────────────────────────────

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

    /// 构造 API URL
    fn build_url(&self, project_id: &str, action: &str) -> String {
        let location = self.location();
        let default_base = vertex_base_url(location);
        let base = self.config.base_url.as_deref().unwrap_or(&default_base);
        format!(
            "{}/{}/projects/{}/locations/{}/publishers/google/models/{}:{}",
            base, VERTEX_API_VERSION, project_id, location, self.config.model, action
        )
    }
}

#[async_trait]
impl LlmProvider for VertexProvider {
    fn id(&self) -> &str {
        "vertex"
    }

    fn auth(&self) -> &Auth {
        &self.auth_enum
    }

    fn supported_models(&self) -> &[&str] {
        &[
            "gemini-1.5-pro",
            "gemini-1.5-flash",
            "gemini-2.0-flash",
            "gemini-2.5-flash",
            "gemini-2.5-pro",
        ]
    }

    fn compile(&self, primitive: &PrimitiveRequest) -> serde_json::Value {
        compile_request(primitive)
    }

    async fn complete(&self, body: serde_json::Value) -> crate::Result<LlmResponse> {
        let token = self.get_access_token().await?;
        let project_id = self.project_id()?;
        let url = self.build_url(&project_id, "generateContent");

        let resp = self
            .http
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", token))
            .json(&body)
            .send()
            .await
            .map_err(|e| crate::Error::Http(e.to_string()))?;

        let status = resp.status();
        let raw_text = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(crate::Error::Provider(
                crate::provider::ProviderError::from_http_status(
                    status.as_u16(),
                    format!("vertex: generateContent failed ({}): {}", status, raw_text.trim()),
                ),
            ));
        }

        parse_response(&raw_text)
    }

    async fn stream(&self, body: serde_json::Value) -> crate::Result<BoxStream<'_, crate::Result<LlmChunk>>> {
        let token = self.get_access_token().await?;
        let project_id = self.project_id()?;
        let url = format!("{}?alt=sse", self.build_url(&project_id, "streamGenerateContent"));

        let resp = self
            .http
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", token))
            .header("Accept", "text/event-stream")
            .json(&body)
            .send()
            .await
            .map_err(|e| crate::Error::Http(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(crate::Error::Provider(
                crate::provider::ProviderError::from_http_status(
                    status.as_u16(),
                    format!("vertex: streamGenerateContent failed ({}): {}", status, text.trim()),
                ),
            ));
        }

        Ok(parse_sse_stream(resp))
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
        let provider = VertexProvider::from_service_account(
            sa_json.to_string(),
            "gemini-2.5-flash".to_string(),
            Some("us-west1".to_string()),
        );
        assert_eq!(provider.config.model, "gemini-2.5-flash");
        assert_eq!(provider.config.location, Some("us-west1".to_string()));
    }

    #[test]
    fn test_build_url() {
        let config = VertexConfig {
            credentials_json: r#"{"project_id":"my-proj","client_email":"e@x.iam.gserviceaccount.com","private_key":"","private_key_id":""}"#.to_string(),
            location: Some("us-central1".to_string()),
            model: "gemini-2.5-flash".to_string(),
            base_url: None,
        };
        let provider = VertexProvider::new(config);
        let url = provider.build_url("my-proj", "generateContent");
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
        let provider = VertexProvider::new(config);
        let url = provider.build_url("my-proj", "generateContent");
        assert_eq!(
            url,
            "https://custom.vertex.api/v1/projects/my-proj/locations/us-central1/publishers/google/models/gemini-2.5-flash:generateContent"
        );
    }
}
