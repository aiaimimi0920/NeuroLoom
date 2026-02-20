//! Vertex AI (Gemini on GCP) Provider 实现
//!
//! 对齐 CLIProxyAPI 参考实现 `gemini_vertex_executor.go`，支持：
//! - Service Account JSON 认证（RS256 JWT → Bearer token）
//! - API Key 认证（x-goog-api-key header）
//! - 非流式 generateContent
//! - 流式 streamGenerateContent（SSE）
//! - 标准 Vertex AI URL 格式

use crate::prompt_ast::PromptAst;
use crate::provider::black_magic_proxy::BlackMagicProxySpec;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

// ── 常量 ────────────────────────────────────────────────────────────────────────
const VERTEX_API_VERSION: &str = "v1";
const VERTEX_TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const VERTEX_CLOUD_PLATFORM_SCOPE: &str =
    "https://www.googleapis.com/auth/cloud-platform";
const VERTEX_DEFAULT_LOCATION: &str = "us-central1";


// ── 数据结构 ────────────────────────────────────────────────────────────────────
/// Vertex provider 配置：选择 SA JSON 或 API key 任一认证方式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexConfig {
    /// 服务账号 JSON 字符串（完整内容）。与 api_key 二选一。
    pub service_account_json: Option<String>,
    /// Google API Key。与 service_account_json 二选一。
    pub api_key: Option<String>,
    /// 区域，默认 "us-central1"
    pub location: Option<String>,
    /// 模型，如 "gemini-2.5-flash"
    pub model: String,
}

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


pub struct VertexProvider {
    config: VertexConfig,
    client: reqwest::Client,
}

// ── 主实现 ───────────────────────────────────────────────────────────────────────
impl VertexProvider {
    pub fn new(config: VertexConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    /// 以 SA JSON 字符串构建，指定模型和区域
    pub fn from_service_account(
        service_account_json: String,
        model: String,
        location: Option<String>,
    ) -> Self {
        Self::new(VertexConfig {
            service_account_json: Some(service_account_json),
            api_key: None,
            location,
            model,
        })
    }

    /// 以 API key 构建
    pub fn from_api_key(api_key: String, model: String) -> Self {
        Self::new(VertexConfig {
            service_account_json: None,
            api_key: Some(api_key),
            location: None,
            model,
        })
    }

    pub fn get_spec(&self) -> BlackMagicProxySpec {
        use crate::provider::black_magic_proxy::{
            BlackMagicProxyTarget, ProxyExposure, ProxyExposureKind,
        };
        let location = self
            .config
            .location
            .as_deref()
            .unwrap_or(VERTEX_DEFAULT_LOCATION);
        BlackMagicProxySpec {
            target: BlackMagicProxyTarget::Vertex,
            default_base_url: vertex_base_url(location),
            exposures: vec![ProxyExposure {
                kind: ProxyExposureKind::Api,
                path: format!(
                    "/{}/projects/{{project}}/locations/{}/publishers/google/models/{{model}}:streamGenerateContent",
                    VERTEX_API_VERSION, location
                ),
                method: "POST".to_string(),
                auth_header: Some("Authorization".to_string()),
                auth_prefix: Some("Bearer ".to_string()),
                cli_command: None,
                cli_args: vec![],
                notes: "Vertex AI Gemini 流式生成接口".to_string(),
            }],
            notes: "Vertex AI (Google Cloud Gemini) provider".to_string(),
        }
    }

    // ── 认证 ─────────────────────────────────────────────────────────────────────

    /// 获取 Bearer access token
    /// - SA 模式：签发 JWT → POST 换取 access_token
    /// - API key 模式：直接返回 api_key（用作 x-goog-api-key 头）
    async fn get_access_token(&self) -> crate::Result<String> {
        // API key 模式
        if let Some(ref api_key) = self.config.api_key {
            if !api_key.is_empty() {
                return Ok(api_key.clone());
            }
        }

        // Service Account 模式
        let sa_json = self
            .config
            .service_account_json
            .as_deref()
            .ok_or_else(|| {
                crate::NeuroLoomError::LlmProvider(
                    "vertex: no service_account_json or api_key provided".to_string(),
                )
            })?;

        self.exchange_jwt_for_token(sa_json).await
    }

    /// 解析 SA JSON，签发 JWT，向 Google token 端点换取 access_token
    async fn exchange_jwt_for_token(&self, sa_json: &str) -> crate::Result<String> {
        // 1. 解析服务账号 JSON
        let sa: ServiceAccount = serde_json::from_str(sa_json).map_err(|e| {
            crate::NeuroLoomError::LlmProvider(format!(
                "vertex: failed to parse service account JSON: {}",
                e
            ))
        })?;

        // 2. 规范化私钥（对齐 keyutil.go NormalizeServiceAccountJSON）
        let pem = normalize_private_key(&sa.private_key).map_err(|e| {
            crate::NeuroLoomError::LlmProvider(format!(
                "vertex: invalid private key: {}",
                e
            ))
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
            crate::NeuroLoomError::LlmProvider(format!(
                "vertex: failed to load RSA private key: {}",
                e
            ))
        })?;
        let jwt = encode(&Header::new(Algorithm::RS256), &claims, &encoding_key).map_err(|e| {
            crate::NeuroLoomError::LlmProvider(format!(
                "vertex: failed to sign JWT: {}",
                e
            ))
        })?;

        // 5. POST 换取 access_token
        let grant_type = "urn:ietf:params:oauth:grant-type:jwt-bearer";
        let params = [("grant_type", grant_type), ("assertion", jwt.as_str())];
        let resp = self
            .client
            .post(VERTEX_TOKEN_ENDPOINT)
            .form(&params)
            .send()
            .await
            .map_err(|e| {
                crate::NeuroLoomError::LlmProvider(format!(
                    "vertex: token exchange request failed: {}",
                    e
                ))
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(crate::NeuroLoomError::LlmProvider(format!(
                "vertex: token exchange failed ({}): {}",
                status,
                body.trim()
            )));
        }

        let token_resp: GoogleTokenResponse = resp.json().await.map_err(|e| {
            crate::NeuroLoomError::LlmProvider(format!(
                "vertex: failed to parse token response: {}",
                e
            ))
        })?;

        Ok(token_resp.access_token)
    }

    // ── Request 构建 ─────────────────────────────────────────────────────────────

    /// 将 PromptAst 编译为 Gemini/Vertex JSON 请求体
    ///
    /// Vertex 格式与 Antigravity/GeminiCli 相同的 Gemini native 格式：
    /// - role: "user" | "model"（不用 "assistant"）
    /// - parts: [{ "text": "..." }]
    /// - systemInstruction: { "parts": [...] }
    pub fn compile_request(&self, ast: &PromptAst) -> Value {
        let openai_msgs = ast.to_openai_messages();

        let mut system_parts: Vec<Value> = Vec::new();
        let mut contents: Vec<Value> = Vec::new();

        for msg in &openai_msgs {
            let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("user");
            let text = msg.get("content").and_then(|c| c.as_str()).unwrap_or("");

            match role {
                "system" => {
                    if !text.is_empty() {
                        system_parts.push(serde_json::json!({ "text": text }));
                    }
                }
                "assistant" => {
                    contents.push(serde_json::json!({
                        "role": "model",
                        "parts": [{ "text": text }]
                    }));
                }
                _ => {
                    contents.push(serde_json::json!({
                        "role": "user",
                        "parts": [{ "text": text }]
                    }));
                }
            }
        }

        if contents.is_empty() && !system_parts.is_empty() {
            contents.push(serde_json::json!({
                "role": "user",
                "parts": [{ "text": "" }]
            }));
        }

        let mut body = serde_json::json!({ "contents": contents });

        if !system_parts.is_empty() {
            body["systemInstruction"] = serde_json::json!({ "parts": system_parts });
        }

        body
    }

    // ── API 调用 ─────────────────────────────────────────────────────────────────

    /// 从 config 中提取 project_id（SA 模式下必须）
    fn project_id(&self) -> crate::Result<String> {
        if let Some(ref sa_json) = self.config.service_account_json {
            let sa: ServiceAccount = serde_json::from_str(sa_json).map_err(|e| {
                crate::NeuroLoomError::LlmProvider(format!(
                    "vertex: failed to parse service account for project_id: {}",
                    e
                ))
            })?;
            return Ok(sa.project_id);
        }
        // API key 模式不需要 project_id
        Ok(String::new())
    }

    fn location(&self) -> &str {
        self.config
            .location
            .as_deref()
            .unwrap_or(VERTEX_DEFAULT_LOCATION)
    }

    /// 构造 API URL
    ///
    /// - SA 模式：`https://{region}-aiplatform.googleapis.com/v1/projects/{proj}/locations/{region}/publishers/google/models/{model}:{action}`
    /// - API key 模式：`https://generativelanguage.googleapis.com/v1beta/models/{model}:{action}`
    ///   注意：generativelanguage.googleapis.com **不使用** publishers/google/ 路径，
    ///   且版本为 v1beta（v1 不支持部分模型）
    fn build_url(&self, project_id: &str, action: &str) -> String {
        if project_id.is_empty() {
            // API key 模式：走 Google AI (generativelanguage.googleapis.com)
            // 正确格式：/v1beta/models/{model}:{action}
            format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:{}",
                self.config.model, action
            )
        } else {
            // SA 模式：走 Vertex AI (aiplatform.googleapis.com)
            let location = self.location();
            let base = vertex_base_url(location);
            format!(
                "{}/{}/projects/{}/locations/{}/publishers/google/models/{}:{}",
                base, VERTEX_API_VERSION, project_id, location, self.config.model, action
            )
        }
    }

    /// 设置请求认证头（同步，RequestBuilder 是移动所有权的）
    fn set_auth_header(
        &self,
        builder: reqwest::RequestBuilder,
        is_api_key_mode: bool,
        token: &str,
    ) -> reqwest::RequestBuilder {
        if is_api_key_mode {
            builder.header("x-goog-api-key", token)
        } else {
            builder.header("Authorization", format!("Bearer {}", token))
        }
    }

    /// 非流式生成（generateContent）
    pub async fn complete(&self, ast: &PromptAst) -> crate::Result<String> {
        let token = self.get_access_token().await?;
        let project_id = self.project_id()?;
        let is_api_key = self.config.api_key.as_ref().map_or(false, |k| !k.is_empty());
        let url = self.build_url(&project_id, "generateContent");
        let body = self.compile_request(ast);

        let mut req = self
            .client
            .post(&url)
            .header("Content-Type", "application/json");
        req = self.set_auth_header(req, is_api_key, &token);

        let resp = req.json(&body).send().await.map_err(|e| {
            crate::NeuroLoomError::LlmProvider(format!(
                "vertex: generateContent request failed: {}",
                e
            ))
        })?;

        let status = resp.status();
        let raw_text = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(crate::NeuroLoomError::LlmProvider(format!(
                "vertex: generateContent failed ({}): {}",
                status,
                raw_text.trim()
            )));
        }

        parse_gemini_response(&raw_text)
    }

    /// 流式生成（streamGenerateContent?alt=sse），返回拼接后的完整文本
    pub async fn stream_complete(&self, ast: &PromptAst) -> crate::Result<String> {
        let token = self.get_access_token().await?;
        let project_id = self.project_id()?;
        let is_api_key = self.config.api_key.as_ref().map_or(false, |k| !k.is_empty());
        let url = format!(
            "{}?alt=sse",
            self.build_url(&project_id, "streamGenerateContent")
        );
        let body = self.compile_request(ast);

        let mut req = self.client.post(&url).header("Content-Type", "application/json");
        req = self.set_auth_header(req, is_api_key, &token);

        let resp = req.json(&body).send().await.map_err(|e| {
            crate::NeuroLoomError::LlmProvider(format!(
                "vertex: streamGenerateContent request failed: {}",
                e
            ))
        })?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(crate::NeuroLoomError::LlmProvider(format!(
                "vertex: streamGenerateContent failed ({}): {}",
                status,
                text.trim()
            )));
        }

        parse_sse_stream(resp).await
    }

    /// Token 计数（countTokens）
    pub async fn count_tokens(&self, ast: &PromptAst) -> crate::Result<u64> {
        let token = self.get_access_token().await?;
        let project_id = self.project_id()?;
        let is_api_key = self.config.api_key.as_ref().map_or(false, |k| !k.is_empty());
        let url = self.build_url(&project_id, "countTokens");
        let body = self.compile_request(ast);

        let mut req = self
            .client
            .post(&url)
            .header("Content-Type", "application/json");
        req = self.set_auth_header(req, is_api_key, &token);

        let resp = req.json(&body).send().await.map_err(|e| {
            crate::NeuroLoomError::LlmProvider(format!(
                "vertex: countTokens request failed: {}",
                e
            ))
        })?;

        let status = resp.status();
        let raw_text = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(crate::NeuroLoomError::LlmProvider(format!(
                "vertex: countTokens failed ({}): {}",
                status,
                raw_text.trim()
            )));
        }

        let json: Value = serde_json::from_str(&raw_text).map_err(|e| {
            crate::NeuroLoomError::LlmProvider(format!(
                "vertex: countTokens decode failed: {}",
                e
            ))
        })?;
        Ok(json
            .get("totalTokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0))
    }
}

// ── 独立辅助函数 ─────────────────────────────────────────────────────────────────

/// 返回 Vertex AI 的区域 base URL
/// 对齐 vertexBaseURL() in gemini_vertex_executor.go
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
/// 对齐 keyutil.go sanitizePrivateKey()
///
/// SA JSON 中的 private_key 常见两种形态：
/// 1. 正常换行：`-----BEGIN PRIVATE KEY-----\nBASE64...\n-----END PRIVATE KEY-----\n`
/// 2. 字面转义：`-----BEGIN PRIVATE KEY-----\\nBASE64...\\n-----END PRIVATE KEY-----\\n`
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
    if pk.contains("-----BEGIN RSA PRIVATE KEY-----")
        || pk.contains("-----BEGIN PRIVATE KEY-----")
    {
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

/// 解析 Gemini 非流式响应：candidates[0].content.parts[0].text
fn parse_gemini_response(raw: &str) -> crate::Result<String> {
    let json: Value = serde_json::from_str(raw).map_err(|e| {
        crate::NeuroLoomError::LlmProvider(format!(
            "vertex: generateContent decode response failed: {}",
            e
        ))
    })?;

    json.get("candidates")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("content"))
        .and_then(|c| c.get("parts"))
        .and_then(|p| p.get(0))
        .and_then(|p| p.get("text"))
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            crate::NeuroLoomError::LlmProvider(
                "vertex: unexpected generateContent response format".to_string(),
            )
        })
}

/// 解析 SSE 流，拼接所有 chunk 的 text
async fn parse_sse_stream(resp: reqwest::Response) -> crate::Result<String> {
    use futures::StreamExt;

    let mut stream = resp.bytes_stream();
    let mut buffer = String::new();
    let mut result = String::new();

    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(|e| {
            crate::NeuroLoomError::LlmProvider(format!(
                "vertex: stream read error: {}",
                e
            ))
        })?;
        buffer.push_str(&String::from_utf8_lossy(&bytes));

        // 按行处理
        while let Some(pos) = buffer.find('\n') {
            let line = buffer[..pos].trim().to_string();
            buffer = buffer[pos + 1..].to_string();

            if let Some(data) = line.strip_prefix("data: ") {
                let data = data.trim();
                if data == "[DONE]" || data.is_empty() {
                    continue;
                }
                if let Ok(json) = serde_json::from_str::<Value>(data) {
                    if let Some(text) = json
                        .get("candidates")
                        .and_then(|c| c.get(0))
                        .and_then(|c| c.get("content"))
                        .and_then(|c| c.get("parts"))
                        .and_then(|p| p.get(0))
                        .and_then(|p| p.get("text"))
                        .and_then(|t| t.as_str())
                    {
                        result.push_str(text);
                    }
                }
            }
        }
    }

    Ok(result)
}

// ── 测试 ─────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompt_ast::{PromptAst, PromptNode};

    #[test]
    fn test_vertex_base_url() {
        assert_eq!(
            vertex_base_url("us-central1"),
            "https://us-central1-aiplatform.googleapis.com"
        );
        assert_eq!(
            vertex_base_url("global"),
            "https://aiplatform.googleapis.com"
        );
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
        // SA JSON 常见格式：\n 是字面字符串，不是换行
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
    fn test_compile_request_user_message() {
        let config = VertexConfig {
            service_account_json: None,
            api_key: Some("test-key".to_string()),
            location: None,
            model: "gemini-2.5-flash".to_string(),
        };
        let provider = VertexProvider::new(config);
        let ast = PromptAst::new().push(PromptNode::User("Hello Vertex!".to_string()));
        let body = provider.compile_request(&ast);

        let contents = body["contents"].as_array().unwrap();
        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0]["role"], "user");
        assert_eq!(contents[0]["parts"][0]["text"], "Hello Vertex!");
    }

    #[test]
    fn test_compile_request_system_message() {
        let config = VertexConfig {
            service_account_json: None,
            api_key: Some("test-key".to_string()),
            location: None,
            model: "gemini-2.5-flash".to_string(),
        };
        let provider = VertexProvider::new(config);
        let ast = PromptAst::new()
            .push(PromptNode::System("You are a helpful AI.".to_string()))
            .push(PromptNode::User("Hello!".to_string()));
        let body = provider.compile_request(&ast);

        // systemInstruction 应该被提取
        assert!(body.get("systemInstruction").is_some());
        let parts = body["systemInstruction"]["parts"].as_array().unwrap();
        assert_eq!(parts[0]["text"], "You are a helpful AI.");

        // contents 只有 user
        let contents = body["contents"].as_array().unwrap();
        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0]["role"], "user");
    }

    #[test]
    fn test_build_url_sa_mode() {
        let config = VertexConfig {
            service_account_json: Some(r#"{"project_id":"my-proj","client_email":"e@x.iam.gserviceaccount.com","private_key":"","private_key_id":""}"#.to_string()),
            api_key: None,
            location: Some("us-central1".to_string()),
            model: "gemini-2.5-flash".to_string(),
        };
        let provider = VertexProvider::new(config);
        let url = provider.build_url("my-proj", "generateContent");
        assert_eq!(
            url,
            "https://us-central1-aiplatform.googleapis.com/v1/projects/my-proj/locations/us-central1/publishers/google/models/gemini-2.5-flash:generateContent"
        );
    }

    #[test]
    fn test_build_url_api_key_mode() {
        let config = VertexConfig {
            service_account_json: None,
            api_key: Some("my-api-key".to_string()),
            location: None,
            model: "gemini-2.5-flash".to_string(),
        };
        let provider = VertexProvider::new(config);
        let url = provider.build_url("", "generateContent");
        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent"
        );
    }
}
