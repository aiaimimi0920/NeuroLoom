//! Antigravity (Gemini Code Assist) Provider 实现
//!
//! 对齐 CLIProxyAPI 参考实现，支持：
//! - OAuth2 登录与 token 自动刷新
//! - 多 Base URL fallback（daily → sandbox）
//! - "no capacity" 错误检测与重试
//!
//! 容错分层：
//! - Provider 层：多 Base URL fallback、Provider 特定错误
//! - Gateway 层：通用 429/5xx 重试、跨 Provider 降级

use crate::prompt_ast::PromptAst;
use crate::provider::black_magic_proxy::BlackMagicProxySpec;
use chrono::{DateTime, Duration, Utc};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration as StdDuration, SystemTime, UNIX_EPOCH};
use url::Url;
use uuid::Uuid;

// ── 常量定义（对齐 CLIProxyAPI 参考实现）──────────────────────────────────────
// OAuth 凭据从环境变量读取
// 凭据可从 CLIProxyAPI 等开源项目获取，或设置环境变量：
//   ANTIGRAVITY_CLIENT_ID=xxx ANTIGRAVITY_CLIENT_SECRET=xxx

fn get_client_id() -> String {
    std::env::var("ANTIGRAVITY_CLIENT_ID")
        .expect("ANTIGRAVITY_CLIENT_ID environment variable not set")
}

fn get_client_secret() -> String {
    std::env::var("ANTIGRAVITY_CLIENT_SECRET")
        .expect("ANTIGRAVITY_CLIENT_SECRET environment variable not set")
}

const ANTIGRAVITY_TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const ANTIGRAVITY_AUTH_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const ANTIGRAVITY_USERINFO_ENDPOINT: &str =
    "https://www.googleapis.com/oauth2/v1/userinfo?alt=json";

// API 版本
const ANTIGRAVITY_API_VERSION: &str = "v1internal";

const ANTIGRAVITY_REDIRECT_URI: &str = "http://127.0.0.1:51121/oauth-callback";
const ANTIGRAVITY_CALLBACK_PORT: u16 = 51121;

// Client-Metadata 必须是预序列化好的 JSON 字符串，不能再做二次序列化
const ANTIGRAVITY_CLIENT_METADATA: &str =
    r#"{"ideType":"IDE_UNSPECIFIED","platform":"PLATFORM_UNSPECIFIED","pluginType":"GEMINI"}"#;
const ANTIGRAVITY_USER_AGENT: &str = "google-api-nodejs-client/9.15.1";
const ANTIGRAVITY_API_CLIENT: &str = "google-cloud-sdk vscode_cloudshelleditor/0.1";

const ANTIGRAVITY_SCOPES: &[&str] = &[
    "https://www.googleapis.com/auth/cloud-platform",
    "https://www.googleapis.com/auth/userinfo.email",
    "https://www.googleapis.com/auth/userinfo.profile",
    "https://www.googleapis.com/auth/cclog",
    "https://www.googleapis.com/auth/experimentsandconfigs",
];

// ── 多 Base URL 支持（对齐 CLIProxyAPI antigravityBaseURLFallbackOrder）────────────
/// Base URL 列表，按优先级排序：prod 优先，daily/sandbox 作为降级
const ANTIGRAVITY_BASE_URLS: &[&str] = &[
    "https://cloudcode-pa.googleapis.com",
    "https://daily-cloudcode-pa.googleapis.com",
    "https://daily-cloudcode-pa.sandbox.googleapis.com",
];

/// Token 刷新提前量
/// CLIProxyAPI 用 3000s 是因为它是代理服务器处理大量并发请求；
/// 作为直接客户端，5 分钟提前量足够避免 token 过期中断请求。
const TOKEN_REFRESH_LEAD_SECONDS: i64 = 300;

/// "no capacity" 重试配置
const NO_CAPACITY_MAX_RETRIES: usize = 3;
const NO_CAPACITY_BASE_DELAY_MS: u64 = 250;

// ── 数据结构 ───────────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntigravityConfig {
    pub model: String,
    pub token_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntigravityToken {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: DateTime<Utc>,
    pub project_id: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OAuthTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: i64,
    #[allow(dead_code)]
    token_type: Option<String>,
}

/// Provider 执行错误，带有重试信号供 Gateway 层决策
#[derive(Debug)]
pub struct AntigravityError {
    pub message: String,
    /// 是否应该重试（同一 Provider）
    pub retryable: bool,
    /// 是否应该触发跨 Provider 降级
    pub should_fallback: bool,
    /// 建议的重试延迟（毫秒）
    pub retry_after_ms: Option<u64>,
}

impl std::fmt::Display for AntigravityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AntigravityError {}

pub struct AntigravityProvider {
    config: AntigravityConfig,
    client: reqwest::Client,
}

// ── 主实现 ─────────────────────────────────────────────────────────────────────
impl AntigravityProvider {
    pub fn new(config: AntigravityConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    pub fn default_provider() -> Self {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".to_string());
        let token_path =
            PathBuf::from(home).join(".nl_llm").join("antigravity_token.json");
        Self::new(AntigravityConfig {
            model: "gemini-2.5-flash".to_string(),
            token_path,
        })
    }

    pub fn get_spec(&self) -> BlackMagicProxySpec {
        BlackMagicProxySpec {
            target: crate::provider::black_magic_proxy::BlackMagicProxyTarget::Antigravity,
            default_base_url: ANTIGRAVITY_BASE_URLS[0].to_string(),
            exposures: vec![crate::provider::black_magic_proxy::ProxyExposure {
                kind: crate::provider::black_magic_proxy::ProxyExposureKind::Api,
                path: format!("/{}:streamGenerateContent", ANTIGRAVITY_API_VERSION),
                method: "POST".to_string(),
                auth_header: Some("Authorization".to_string()),
                auth_prefix: Some("Bearer ".to_string()),
                cli_command: None,
                cli_args: vec![],
                notes: "Antigravity streamGenerateContent".to_string(),
            }],
            notes: "Gemini Code Assist (Antigravity) provider".to_string(),
        }
    }

    /// 确保 token 有效，必要时刷新，返回 access_token
    pub async fn ensure_authenticated(&self) -> crate::Result<String> {
        let mut token = self.load_token().await?;

        // 使用与 CLIProxyAPI 一致的 50 分钟提前量
        if token.expires_at <= Utc::now() + Duration::seconds(TOKEN_REFRESH_LEAD_SECONDS) {
            token = self.refresh_token_data(&token.refresh_token, &token).await?;
            self.save_token(&token)?;
        }

        // 确保 project_id 存在
        if token.project_id.is_none() {
            match self.fetch_project_id(&token.access_token).await {
                Ok(pid) => {
                    token.project_id = Some(pid);
                    self.save_token(&token)?;
                }
                Err(e) => {
                    // project_id 不是必须的，只是 warn
                    eprintln!("Warning: could not fetch project ID: {:?}", e);
                }
            }
        }

        Ok(token.access_token)
    }

    pub async fn get_auth_headers(&self) -> crate::Result<HeaderMap> {
        let access_token = self.ensure_authenticated().await?;

        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&format!("Bearer {}", access_token)).unwrap(),
        );
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        headers.insert("User-Agent", HeaderValue::from_static(ANTIGRAVITY_USER_AGENT));
        headers.insert(
            "X-Goog-Api-Client",
            HeaderValue::from_static(ANTIGRAVITY_API_CLIENT),
        );
        headers.insert(
            "Client-Metadata",
            HeaderValue::from_static(ANTIGRAVITY_CLIENT_METADATA),
        );
        Ok(headers)
    }

    pub fn compile_request(&self, ast: &PromptAst) -> Value {
        // Antigravity/Gemini 的 contents 格式与 OpenAI 不同：
        //   OpenAI: { "role": "user", "content": "text" }
        //   Gemini: { "role": "user", "parts": [{ "text": "text" }] }
        //
        // Gemini 不支持 "system" role，system 消息放入 request.systemInstruction
        let openai_msgs = ast.to_openai_messages();

        let mut system_parts: Vec<serde_json::Value> = Vec::new();
        let mut contents: Vec<serde_json::Value> = Vec::new();

        for msg in &openai_msgs {
            let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("user");
            let text = msg.get("content").and_then(|c| c.as_str()).unwrap_or("");

            match role {
                "system" => {
                    // 收集 system 内容到 systemInstruction.parts
                    if !text.is_empty() {
                        system_parts.push(serde_json::json!({ "text": text }));
                    }
                }
                "assistant" => {
                    // Gemini 用 "model" 而不是 "assistant"
                    contents.push(serde_json::json!({
                        "role": "model",
                        "parts": [{ "text": text }]
                    }));
                }
                _ => {
                    // user 消息
                    contents.push(serde_json::json!({
                        "role": "user",
                        "parts": [{ "text": text }]
                    }));
                }
            }
        }

        // 如果没有任何 contents（只有 system），补一条空 user
        if contents.is_empty() && !system_parts.is_empty() {
            contents.push(serde_json::json!({
                "role": "user",
                "parts": [{ "text": "" }]
            }));
        }

        // 生成 stable session ID（对齐参考实现 generateStableSessionID）
        let session_id = generate_session_id(&contents);
        let request_id = format!("agent-{}", Uuid::new_v4());

        // 从已保存的 token 读取 project_id（若有的话）
        let project = self.load_project_id().unwrap_or_else(|| generate_project_id());

        // 构建请求体
        let mut request_inner = serde_json::json!({
            "contents": contents,
            "sessionId": session_id
        });

        // 如果有 system 消息，使用 Gemini 原生 systemInstruction 字段
        if !system_parts.is_empty() {
            request_inner["systemInstruction"] = serde_json::json!({
                "parts": system_parts
            });
        }

        // 防御性清理：删除可能引起问题的字段（对齐参考实现）
        if let Some(obj) = request_inner.as_object_mut() {
            obj.remove("safetySettings");
            if let Some(gen_config) = obj.get_mut("generationConfig") {
                if let Some(gc_obj) = gen_config.as_object_mut() {
                    gc_obj.remove("maxOutputTokens");
                }
            }
        }

        // 移动顶层 toolConfig 到 request.toolConfig（对齐 CLIProxyAPI geminiToAntigravity）
        let tool_config = request_inner.get("toolConfig").cloned();
        if let Some(tc) = tool_config {
            if request_inner.get("request").and_then(|r| r.get("toolConfig")).is_none() {
                request_inner["toolConfig"] = tc;
            }
            if let Some(obj) = request_inner.as_object_mut() {
                obj.remove("toolConfig");
            }
        }

        serde_json::json!({
            "model": self.config.model,
            "userAgent": "antigravity",
            "requestType": "agent",
            "project": project,
            "requestId": request_id,
            "request": request_inner
        })
    }

    /// 从 token 文件读取 project_id
    fn load_project_id(&self) -> Option<String> {
        if self.config.token_path.exists() {
            let content = fs::read_to_string(&self.config.token_path).ok()?;
            let token: AntigravityToken = serde_json::from_str(&content).ok()?;
            token.project_id.filter(|s| !s.is_empty())
        } else {
            None
        }
    }

    /// 执行非流式聊天补全（带多 Base URL fallback 和 "no capacity" 重试）
    ///
    /// 容错策略（Provider 层）：
    /// 1. 遍历所有 Base URL（prod → daily → sandbox）
    /// 2. 遇到 429 或 "no capacity" 时尝试下一个 URL
    /// 3. "no capacity" 错误会触发延迟重试
    ///
    /// 调用 `generateContent` 端点，返回模型生成的文本。
    /// Gemini 格式响应: candidates[0].content.parts[0].text
    pub async fn complete(&self, ast: &PromptAst) -> crate::Result<String> {
        let access_token = self.ensure_authenticated().await?;
        let body = self.compile_request(ast);

        for _retry_attempt in 0..NO_CAPACITY_MAX_RETRIES {
            for (url_idx, base_url) in ANTIGRAVITY_BASE_URLS.iter().enumerate() {
                let url = format!(
                    "{}/{}:generateContent",
                    base_url, ANTIGRAVITY_API_VERSION
                );

                let resp = self
                    .client
                    .post(&url)
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .header("User-Agent", ANTIGRAVITY_USER_AGENT)
                    .header("X-Goog-Api-Client", ANTIGRAVITY_API_CLIENT)
                    .header("Client-Metadata", ANTIGRAVITY_CLIENT_METADATA)
                    .json(&body)
                    .send()
                    .await
                    .map_err(|e| {
                        crate::NeuroLoomError::LlmProvider(format!(
                            "antigravity generateContent request failed: {}",
                            e
                        ))
                    })?;

                let status = resp.status();
                let raw_text = resp.text().await.unwrap_or_default();

                // 成功响应
                if status.is_success() {
                    return Self::parse_generate_response(&raw_text);
                }

                // "no capacity" 或 429：尝试下一个 Base URL
                if Self::is_no_capacity_error(status, &raw_text) || status.as_u16() == 429 {
                    if url_idx + 1 < ANTIGRAVITY_BASE_URLS.len() {
                        continue; // 还有 URL 可试
                    }
                    // 所有 URL 都试过了
                    if Self::is_no_capacity_error(status, &raw_text) {
                        // "no capacity" 延迟后进入下一轮重试
                        let delay = Self::no_capacity_delay(_retry_attempt);
                        tokio::time::sleep(StdDuration::from_millis(delay)).await;
                        break; // 跳出 URL 循环，进入下一次重试
                    }
                    // 429 在所有 URL 上都触发，返回错误让 Gateway 层处理
                    return Err(crate::NeuroLoomError::LlmProvider(format!(
                        "antigravity rate limited on all base URLs: {}",
                        raw_text.trim()
                    )));
                }

                // 其他错误直接返回
                return Err(crate::NeuroLoomError::LlmProvider(format!(
                    "antigravity generateContent failed ({}): {}",
                    status,
                    raw_text.trim()
                )));
            }
        }

        Err(crate::NeuroLoomError::LlmProvider(
            "antigravity: max retries exceeded for 'no capacity' errors".to_string(),
        ))
    }

    /// 解析 generateContent 响应
    fn parse_generate_response(raw_text: &str) -> crate::Result<String> {
        let json: Value = serde_json::from_str(raw_text).map_err(|e| {
            crate::NeuroLoomError::LlmProvider(format!(
                "antigravity generateContent: decode response failed: {}",
                e
            ))
        })?;

        // Antigravity/Gemini 响应结构（两种可能）:
        // 形式 1: { "response": { "candidates": [...] } }
        // 形式 2: { "candidates": [...] }
        let candidates = json
            .get("response")
            .and_then(|r| r.get("candidates"))
            .or_else(|| json.get("candidates"));

        candidates
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("content"))
            .and_then(|c| c.get("parts"))
            .and_then(|p| p.get(0))
            .and_then(|p| p.get("text"))
            .and_then(|t| t.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| {
                crate::NeuroLoomError::LlmProvider(format!(
                    "antigravity generateContent: unexpected response format"
                ))
            })
    }

    /// 检测 "no capacity" 错误（对齐 CLIProxyAPI antigravityShouldRetryNoCapacity）
    fn is_no_capacity_error(status: reqwest::StatusCode, body: &str) -> bool {
        status.as_u16() == 503 && body.to_lowercase().contains("no capacity available")
    }

    /// 计算 "no capacity" 重试延迟（对齐 CLIProxyAPI antigravityNoCapacityRetryDelay）
    fn no_capacity_delay(attempt: usize) -> u64 {
        let delay = (attempt + 1) as u64 * NO_CAPACITY_BASE_DELAY_MS;
        delay.min(2000) // 最大 2 秒
    }

    /// 执行流式聊天补全，返回完整拼接文本（SSE 格式）
    ///
    /// 容错策略与 complete() 相同：多 Base URL fallback + "no capacity" 重试
    ///
    /// 调用 `streamGenerateContent` 端点，按行解析 SSE，
    /// 提取每个 chunk 的 candidates[0].content.parts[0].text 并拼接。
    pub async fn stream_complete(&self, ast: &PromptAst) -> crate::Result<String> {
        let access_token = self.ensure_authenticated().await?;
        let body = self.compile_request(ast);

        for _retry_attempt in 0..NO_CAPACITY_MAX_RETRIES {
            for (url_idx, base_url) in ANTIGRAVITY_BASE_URLS.iter().enumerate() {
                let url = format!(
                    "{}/{}:streamGenerateContent?alt=sse",
                    base_url, ANTIGRAVITY_API_VERSION
                );

                let resp = self
                    .client
                    .post(&url)
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .header("Accept", "text/event-stream")
                    .header("User-Agent", ANTIGRAVITY_USER_AGENT)
                    .header("X-Goog-Api-Client", ANTIGRAVITY_API_CLIENT)
                    .header("Client-Metadata", ANTIGRAVITY_CLIENT_METADATA)
                    .json(&body)
                    .send()
                    .await
                    .map_err(|e| {
                        crate::NeuroLoomError::LlmProvider(format!(
                            "antigravity streamGenerateContent request failed: {}",
                            e
                        ))
                    })?;

                let status = resp.status();

                if status.is_success() {
                    return Self::parse_stream_response(resp).await;
                }

                let text = resp.text().await.unwrap_or_default();

                // "no capacity" 或 429：尝试下一个 Base URL
                if Self::is_no_capacity_error(status, &text) || status.as_u16() == 429 {
                    if url_idx + 1 < ANTIGRAVITY_BASE_URLS.len() {
                        continue;
                    }
                    if Self::is_no_capacity_error(status, &text) {
                        let delay = Self::no_capacity_delay(_retry_attempt);
                        tokio::time::sleep(StdDuration::from_millis(delay)).await;
                        break;
                    }
                    return Err(crate::NeuroLoomError::LlmProvider(format!(
                        "antigravity streamGenerateContent rate limited: {}",
                        text.trim()
                    )));
                }

                return Err(crate::NeuroLoomError::LlmProvider(format!(
                    "antigravity streamGenerateContent failed ({}): {}",
                    status,
                    text.trim()
                )));
            }
        }

        Err(crate::NeuroLoomError::LlmProvider(
            "antigravity: max retries exceeded for 'no capacity' errors".to_string(),
        ))
    }

    /// 解析流式响应
    async fn parse_stream_response(resp: reqwest::Response) -> crate::Result<String> {
        let raw = resp.text().await.map_err(|e| {
            crate::NeuroLoomError::LlmProvider(format!(
                "antigravity streamGenerateContent: read body failed: {}",
                e
            ))
        })?;

        // 逐行解析，拼接所有 text 片段
        let mut result = String::new();
        for line in raw.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            // 跳过 SSE 前缀 "data: "
            let json_str = line.strip_prefix("data: ").unwrap_or(line);
            if let Ok(chunk) = serde_json::from_str::<Value>(json_str) {
                // 尝试两种响应格式
                let text = chunk
                    .get("response")
                    .and_then(|r| r.get("candidates"))
                    .or_else(|| chunk.get("candidates"))
                    .and_then(|c| c.get(0))
                    .and_then(|c| c.get("content"))
                    .and_then(|c| c.get("parts"))
                    .and_then(|p| p.get(0))
                    .and_then(|p| p.get("text"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("");
                result.push_str(text);
            }
        }

        if result.is_empty() {
            return Err(crate::NeuroLoomError::LlmProvider(
                "antigravity streamGenerateContent: no text content in response".to_string(),
            ));
        }

        Ok(result)
    }

    // ── countTokens API ─────────────────────────────────────────────────────

    /// 估算 token 数量（带多 Base URL fallback）
    pub async fn count_tokens(&self, ast: &PromptAst) -> crate::Result<u64> {
        let access_token = self.ensure_authenticated().await?;
        let body = self.compile_request(ast);

        // 移除不需要的字段
        let body = {
            let mut b = body;
            if let Some(obj) = b.as_object_mut() {
                obj.remove("project");
                obj.remove("model");
                if let Some(req) = obj.get_mut("request") {
                    if let Some(req_obj) = req.as_object_mut() {
                        req_obj.remove("safetySettings");
                    }
                }
            }
            b
        };

        for base_url in ANTIGRAVITY_BASE_URLS {
            let url = format!(
                "{}/{}:countTokens",
                base_url, ANTIGRAVITY_API_VERSION
            );

            let resp = self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .header("User-Agent", ANTIGRAVITY_USER_AGENT)
                .header("X-Goog-Api-Client", ANTIGRAVITY_API_CLIENT)
                .header("Client-Metadata", ANTIGRAVITY_CLIENT_METADATA)
                .json(&body)
                .send()
                .await
                .map_err(|e| {
                    crate::NeuroLoomError::LlmProvider(format!(
                        "antigravity countTokens request failed: {}",
                        e
                    ))
                })?;

            let status = resp.status();
            let raw = resp.text().await.unwrap_or_default();

            if status.is_success() {
                let json: Value = serde_json::from_str(&raw).map_err(|e| {
                    crate::NeuroLoomError::LlmProvider(format!(
                        "antigravity countTokens: decode response failed: {}",
                        e
                    ))
                })?;

                let total = json
                    .get("response")
                    .and_then(|r| r.get("totalTokens"))
                    .or_else(|| json.get("totalTokens"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);

                return Ok(total);
            }

            // 429/503：尝试下一个 Base URL
            if status.as_u16() == 429 || status.as_u16() == 503 {
                continue;
            }

            return Err(crate::NeuroLoomError::LlmProvider(format!(
                "antigravity countTokens failed ({}): {}",
                status,
                raw.trim()
            )));
        }

        Err(crate::NeuroLoomError::LlmProvider(
            "antigravity countTokens failed on all base URLs".to_string(),
        ))
    }

    // ── fetchAvailableModels API ─────────────────────────────────────────────

    /// 查询当前账户可用的 Antigravity 模型列表（带多 Base URL fallback）
    pub async fn fetch_available_models(&self) -> crate::Result<Vec<String>> {
        let access_token = self.ensure_authenticated().await?;

        let body = serde_json::json!({
            "metadata": {
                "ideType": "ANTIGRAVITY",
                "platform": "PLATFORM_UNSPECIFIED",
                "pluginType": "GEMINI"
            }
        });

        for base_url in ANTIGRAVITY_BASE_URLS {
            let url = format!(
                "{}/{}:fetchAvailableModels",
                base_url, ANTIGRAVITY_API_VERSION
            );

            let resp = self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .header("User-Agent", ANTIGRAVITY_USER_AGENT)
                .header("X-Goog-Api-Client", ANTIGRAVITY_API_CLIENT)
                .header("Client-Metadata", ANTIGRAVITY_CLIENT_METADATA)
                .json(&body)
                .send()
                .await
                .map_err(|e| {
                    crate::NeuroLoomError::LlmProvider(format!(
                        "antigravity fetchAvailableModels request failed: {}",
                        e
                    ))
                })?;

            let status = resp.status();
            let raw = resp.text().await.unwrap_or_default();

            if status.is_success() {
                let json: Value = serde_json::from_str(&raw).map_err(|e| {
                    crate::NeuroLoomError::LlmProvider(format!(
                        "antigravity fetchAvailableModels: decode response failed: {}",
                        e
                    ))
                })?;

                let models_array = json
                    .get("response")
                    .and_then(|r| r.get("models"))
                    .or_else(|| json.get("models"))
                    .and_then(|v| v.as_array());

                let names: Vec<String> = models_array
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|m| {
                                m.get("name")
                                    .or_else(|| m.get("modelName"))
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string())
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                return Ok(names);
            }

            // 429/503：尝试下一个 Base URL
            if status.as_u16() == 429 || status.as_u16() == 503 {
                continue;
            }

            return Err(crate::NeuroLoomError::LlmProvider(format!(
                "antigravity fetchAvailableModels failed ({}): {}",
                status,
                raw.trim()
            )));
        }

        Err(crate::NeuroLoomError::LlmProvider(
            "antigravity fetchAvailableModels failed on all base URLs".to_string(),
        ))
    }

    // ── 内部：Token 文件 I/O ────────────────────────────────────────────────

    async fn load_token(&self) -> crate::Result<AntigravityToken> {
        if self.config.token_path.exists() {
            let content = fs::read_to_string(&self.config.token_path)?;
            let token: AntigravityToken = serde_json::from_str(&content)?;
            return Ok(token);
        }
        // 文件不存在，发起 OAuth 登录
        self.login().await
    }

    fn save_token(&self, token: &AntigravityToken) -> crate::Result<()> {
        if let Some(parent) = self.config.token_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(token)?;
        fs::write(&self.config.token_path, content)?;
        Ok(())
    }

    // ── 内部：OAuth 登录流程 ────────────────────────────────────────────────

    async fn login(&self) -> crate::Result<AntigravityToken> {
        // 1. 构造 state 和 Auth URL
        let state = format!(
            "{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let client_id = get_client_id();
        let auth_url = Url::parse_with_params(
            ANTIGRAVITY_AUTH_ENDPOINT,
            &[
                ("access_type", "offline"),
                ("client_id", client_id.as_str()),
                ("prompt", "consent"),
                ("redirect_uri", ANTIGRAVITY_REDIRECT_URI),
                ("response_type", "code"),
                ("scope", &ANTIGRAVITY_SCOPES.join(" ")),
                ("state", &state),
            ],
        )
        .unwrap();

        eprintln!("\n=== Antigravity OAuth Login ===");
        eprintln!("请在浏览器中打开以下链接完成登录:\n{}\n", auth_url);

        // 2. 本地回调服务器等待 code
        let (tx, rx) = std::sync::mpsc::channel();
        let server =
            tiny_http::Server::http(format!("127.0.0.1:{}", ANTIGRAVITY_CALLBACK_PORT))
                .map_err(|e| {
                    crate::NeuroLoomError::LlmProvider(format!(
                        "Failed to start callback server: {}",
                        e
                    ))
                })?;

        std::thread::spawn(move || {
            for request in server.incoming_requests() {
                let url = request.url().to_string();
                if url.contains("code=") {
                    let code = url
                        .split("code=")
                        .nth(1)
                        .unwrap_or("")
                        .split('&')
                        .next()
                        .unwrap_or("")
                        .to_string();
                    let html = "<h1>Login successful</h1><p>You can close this window.</p>";
                    let response = tiny_http::Response::from_string(html)
                        .with_header(
                            "Content-Type: text/html"
                                .parse::<tiny_http::Header>()
                                .unwrap(),
                        );
                    let _ = request.respond(response);
                    let _ = tx.send(Ok(code));
                    break;
                } else if url.contains("error=") {
                    let err = url
                        .split("error=")
                        .nth(1)
                        .unwrap_or("unknown")
                        .split('&')
                        .next()
                        .unwrap_or("unknown")
                        .to_string();
                    let _ = request.respond(tiny_http::Response::from_string(
                        "<h1>Login failed</h1><p>Please check the CLI output.</p>",
                    ));
                    let _ = tx.send(Err(err));
                    break;
                }
            }
        });

        eprintln!("Waiting for callback on port {}...", ANTIGRAVITY_CALLBACK_PORT);
        let code = match rx.recv() {
            Ok(Ok(code)) => code,
            Ok(Err(e)) => {
                return Err(crate::NeuroLoomError::LlmProvider(format!(
                    "OAuth error returned: {}",
                    e
                )))
            }
            Err(_) => {
                return Err(crate::NeuroLoomError::LlmProvider(
                    "Failed to receive auth code from callback server".to_string(),
                ))
            }
        };

        // 3. 用 code 换 token
        let token_resp = self.exchange_code(&code).await?;

        let access_token = token_resp.access_token.clone();
        let refresh_token = token_resp.refresh_token.ok_or_else(|| {
            crate::NeuroLoomError::LlmProvider("No refresh_token in token response".to_string())
        })?;
        let expires_at = Utc::now() + Duration::seconds(token_resp.expires_in);

        // 4. 获取用户 email
        let email = self.fetch_user_email(&access_token).await.ok();

        // 5. 获取 Project ID（可选，失败不中断）
        let project_id = self.fetch_project_id(&access_token).await.ok();

        let token = AntigravityToken {
            access_token,
            refresh_token,
            expires_at,
            project_id,
            email,
        };

        self.save_token(&token)?;
        eprintln!("Token saved to {:?}", self.config.token_path);
        Ok(token)
    }

    async fn exchange_code(&self, code: &str) -> crate::Result<OAuthTokenResponse> {
        let client_id = get_client_id();
        let client_secret = get_client_secret();
        let params = [
            ("code", code),
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("redirect_uri", ANTIGRAVITY_REDIRECT_URI),
            ("grant_type", "authorization_code"),
        ];

        let res = self
            .client
            .post(ANTIGRAVITY_TOKEN_ENDPOINT)
            .form(&params)
            .send()
            .await
            .map_err(|e| {
                crate::NeuroLoomError::LlmProvider(format!("Token exchange request failed: {}", e))
            })?;

        if !res.status().is_success() {
            let body = res.text().await.unwrap_or_default();
            return Err(crate::NeuroLoomError::LlmProvider(format!(
                "Token exchange failed ({}): {}",
                body.len(),
                body
            )));
        }

        res.json::<OAuthTokenResponse>().await.map_err(|e| {
            crate::NeuroLoomError::LlmProvider(format!(
                "Failed to parse token response: {}",
                e
            ))
        })
    }

    async fn refresh_token_data(
        &self,
        refresh_token: &str,
        existing: &AntigravityToken,
    ) -> crate::Result<AntigravityToken> {
        let client_id = get_client_id();
        let client_secret = get_client_secret();
        let params = [
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ];

        let res = self
            .client
            .post(ANTIGRAVITY_TOKEN_ENDPOINT)
            .form(&params)
            .send()
            .await
            .map_err(|e| {
                crate::NeuroLoomError::LlmProvider(format!("Token refresh request failed: {}", e))
            })?;

        if !res.status().is_success() {
            let body = res.text().await.unwrap_or_default();
            return Err(crate::NeuroLoomError::LlmProvider(format!(
                "Token refresh failed: {}",
                body
            )));
        }

        let token_resp = res.json::<OAuthTokenResponse>().await.map_err(|e| {
            crate::NeuroLoomError::LlmProvider(format!(
                "Failed to parse refresh token response: {}",
                e
            ))
        })?;

        Ok(AntigravityToken {
            access_token: token_resp.access_token,
            refresh_token: token_resp
                .refresh_token
                .unwrap_or_else(|| refresh_token.to_string()),
            expires_at: Utc::now() + Duration::seconds(token_resp.expires_in),
            project_id: existing.project_id.clone(),
            email: existing.email.clone(),
        })
    }

    // ── 内部：用户信息 ──────────────────────────────────────────────────────

    async fn fetch_user_email(&self, access_token: &str) -> crate::Result<String> {
        let res = self
            .client
            .get(ANTIGRAVITY_USERINFO_ENDPOINT)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await
            .map_err(|e| {
                crate::NeuroLoomError::LlmProvider(format!("UserInfo request failed: {}", e))
            })?;

        let json: Value = res.json().await.map_err(|e| {
            crate::NeuroLoomError::LlmProvider(format!("Failed to parse UserInfo: {}", e))
        })?;

        json.get("email")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| {
                crate::NeuroLoomError::LlmProvider("No email in UserInfo response".to_string())
            })
    }

    // ── 内部：loadCodeAssist + OnboardUser ──────────────────────────────────

    async fn fetch_project_id(&self, access_token: &str) -> crate::Result<String> {
        // 使用第一个 Base URL（登录流程不需要 fallback）
        let base_url = ANTIGRAVITY_BASE_URLS[0];
        let url = format!(
            "{}/{}:loadCodeAssist",
            base_url, ANTIGRAVITY_API_VERSION
        );
        let body = serde_json::json!({
            "metadata": {
                "ideType": "ANTIGRAVITY",
                "platform": "PLATFORM_UNSPECIFIED",
                "pluginType": "GEMINI"
            }
        });

        let res = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .header("User-Agent", ANTIGRAVITY_USER_AGENT)
            .header("X-Goog-Api-Client", ANTIGRAVITY_API_CLIENT)
            .header("Client-Metadata", ANTIGRAVITY_CLIENT_METADATA)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                crate::NeuroLoomError::LlmProvider(format!("loadCodeAssist request failed: {}", e))
            })?;

        let status = res.status();
        let text = res.text().await.map_err(|e| {
            crate::NeuroLoomError::LlmProvider(format!("Failed to read loadCodeAssist body: {}", e))
        })?;

        if !status.is_success() {
            return Err(crate::NeuroLoomError::LlmProvider(format!(
                "loadCodeAssist returned {}: {}",
                status,
                text.trim()
            )));
        }

        let json: Value = serde_json::from_str(&text)?;

        // 解析 project_id（两种可能的结构）
        let mut project_id = String::new();
        if let Some(id) = json.get("cloudaicompanionProject").and_then(|v| v.as_str()) {
            project_id = id.trim().to_string();
        } else if let Some(obj) = json
            .get("cloudaicompanionProject")
            .and_then(|v| v.as_object())
        {
            if let Some(id) = obj.get("id").and_then(|v| v.as_str()) {
                project_id = id.trim().to_string();
            }
        }

        if !project_id.is_empty() {
            return Ok(project_id);
        }

        // project_id 为空时，尝试从 allowedTiers 中获取 tierId，然后调用 onboardUser
        let tier_id = json
            .get("allowedTiers")
            .and_then(|v| v.as_array())
            .and_then(|tiers| {
                tiers.iter().find_map(|t| {
                    if t.get("isDefault").and_then(|v| v.as_bool()).unwrap_or(false) {
                        t.get("id").and_then(|v| v.as_str()).map(|s| s.to_string())
                    } else {
                        None
                    }
                })
            })
            .unwrap_or_else(|| "legacy-tier".to_string());

        self.onboard_user(access_token, &tier_id).await
    }

    async fn onboard_user(
        &self,
        access_token: &str,
        tier_id: &str,
    ) -> crate::Result<String> {
        let base_url = ANTIGRAVITY_BASE_URLS[0];
        let url = format!(
            "{}/{}:onboardUser",
            base_url, ANTIGRAVITY_API_VERSION
        );
        let body = serde_json::json!({
            "tierId": tier_id,
            "metadata": {
                "ideType": "ANTIGRAVITY",
                "platform": "PLATFORM_UNSPECIFIED",
                "pluginType": "GEMINI"
            }
        });

        for _attempt in 1..=5 {
            let res = self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .header("User-Agent", ANTIGRAVITY_USER_AGENT)
                .header("X-Goog-Api-Client", ANTIGRAVITY_API_CLIENT)
                .header("Client-Metadata", ANTIGRAVITY_CLIENT_METADATA)
                .json(&body)
                .send()
                .await
                .map_err(|e| {
                    crate::NeuroLoomError::LlmProvider(format!(
                        "onboardUser request failed: {}",
                        e
                    ))
                })?;

            let status = res.status();
            let text = res.text().await.map_err(|e| {
                crate::NeuroLoomError::LlmProvider(format!(
                    "Failed to read onboardUser body: {}",
                    e
                ))
            })?;

            if !status.is_success() {
                return Err(crate::NeuroLoomError::LlmProvider(format!(
                    "onboardUser returned {}: {}",
                    status,
                    text.trim()
                )));
            }

            let json: Value = serde_json::from_str(&text)?;

            if json.get("done").and_then(|v| v.as_bool()).unwrap_or(false) {
                // 解析 response.cloudaicompanionProject
                let pid = json
                    .get("response")
                    .and_then(|r| r.get("cloudaicompanionProject"))
                    .and_then(|p| match p {
                        Value::String(s) => Some(s.trim().to_string()),
                        Value::Object(o) => o
                            .get("id")
                            .and_then(|v| v.as_str())
                            .map(|s| s.trim().to_string()),
                        _ => None,
                    });

                return pid.ok_or_else(|| {
                    crate::NeuroLoomError::LlmProvider(
                        "onboardUser done but no project_id in response".to_string(),
                    )
                });
            }

            // 还没完成，等 2 秒后重试
            tokio::time::sleep(StdDuration::from_secs(2)).await;
        }

        Err(crate::NeuroLoomError::LlmProvider(
            "onboardUser: max attempts reached without completion".to_string(),
        ))
    }
}

// ── 模块级辅助函数（对齐 CLIProxyAPI 参考实现）────────────────────────────────

/// 生成 stable session ID（对齐 generateStableSessionID）
/// 用第一条 user 消息的 SHA-256 哈希值的前 8 字节作为 session 标识
fn generate_session_id(contents: &[serde_json::Value]) -> String {
    for content in contents {
        if content.get("role").and_then(|r| r.as_str()) == Some("user") {
            if let Some(text) = content
                .get("parts")
                .and_then(|p| p.get(0))
                .and_then(|p| p.get("text"))
                .and_then(|t| t.as_str())
            {
                if !text.is_empty() {
                    let mut hasher = Sha256::new();
                    hasher.update(text.as_bytes());
                    let hash = hasher.finalize();
                    let n = i64::from_be_bytes(hash[..8].try_into().unwrap()) & 0x7FFFFFFFFFFFFFFF;
                    return format!("-{}", n);
                }
            }
        }
    }
    // fallback: 随机 session ID
    let n = rand_i64() & 0x7FFFFFFFFFFFFFFF;
    format!("-{}", n)
}

/// 生成随机 project ID（对齐 generateProjectID）
fn generate_project_id() -> String {
    let adjectives = ["useful", "bright", "swift", "calm", "bold"];
    let nouns = ["fuze", "wave", "spark", "flow", "core"];
    let uid = Uuid::new_v4().to_string();
    let random_part = &uid[..5];
    let adj = adjectives[rand_usize() % adjectives.len()];
    let noun = nouns[rand_usize() % nouns.len()];
    format!("{}-{}-{}", adj, noun, random_part)
}

fn rand_i64() -> i64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    (now as i64).wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407)
}

fn rand_usize() -> usize {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    now as usize
}
