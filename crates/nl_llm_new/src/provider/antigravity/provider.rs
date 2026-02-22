//! Antigravity Provider 实现
//!
//! 使用 Google OAuth 认证，调用 Cloud Code PA API

use super::config::AntigravityConfig;
use crate::auth::providers::antigravity::AntigravityOAuth;
use crate::auth::{Auth, OAuthProvider};
use crate::primitive::PrimitiveRequest;
use crate::provider::{BoxStream, LlmChunk, LlmProvider, LlmResponse, StopReason, Usage};
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue};
use std::time::Duration;
use tokio::sync::Mutex;

/// API 端点
const BASE_URL: &str = "https://cloudcode-pa.googleapis.com";
const API_VERSION: &str = "v1internal";

/// Antigravity Provider
///
/// 通过 Google OAuth 认证，调用 Gemini Code Assist API
pub struct AntigravityProvider {
    config: AntigravityConfig,
    /// 认证实例
    auth: Mutex<AntigravityOAuth>,
    /// 复用的 HTTP Client
    http: reqwest::Client,
    /// Auth enum 用于 trait 方法返回
    auth_enum: Auth,
}

impl AntigravityProvider {
    /// 创建新的 Provider
    pub fn new(config: AntigravityConfig) -> Self {
        let auth = AntigravityOAuth::from_file(&config.token_path)
            .expect("Failed to load AntigravityOAuth");

        let auth_enum = Auth::OAuth {
            provider: OAuthProvider::Antigravity,
            token_path: config.token_path.clone(),
        };

        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            auth: Mutex::new(auth),
            http,
            auth_enum,
        }
    }

    /// 使用默认配置创建 Provider
    pub fn with_default_config(model: String) -> Self {
        Self::new(AntigravityConfig::with_default_path(model))
    }

    /// 确保认证有效
    async fn ensure_auth(&self) -> crate::Result<()> {
        let mut auth_guard = self.auth.lock().await;
        auth_guard
            .ensure_authenticated()
            .await
            .map_err(|e| crate::Error::Auth(e.to_string()))
    }

    /// 获取 Access Token
    async fn get_access_token(&self) -> crate::Result<String> {
        self.ensure_auth().await?;
        let auth_guard = self.auth.lock().await;
        auth_guard
            .access_token()
            .map(|s| s.to_string())
            .ok_or_else(|| crate::Error::Auth("No access token available".to_string()))
    }

    /// 获取 Project ID
    async fn get_project_id(&self) -> crate::Result<Option<String>> {
        let auth_guard = self.auth.lock().await;
        Ok(auth_guard.project_id().map(|s| s.to_string()))
    }

    /// 生成 Project ID
    fn generate_project_id() -> String {
        let adjectives = ["useful", "bright", "swift", "calm", "bold"];
        let nouns = ["fuze", "wave", "spark", "flow", "core"];
        let uid = uuid::Uuid::new_v4().to_string();
        let random_part = &uid[..5];
        let nanos = chrono::Utc::now().timestamp_subsec_nanos() as usize;
        let adj = adjectives[nanos % adjectives.len()];
        let noun = nouns[(nanos / 2) % nouns.len()];
        format!("{}-{}-{}", adj, noun, random_part)
    }


    /// 构建 API 请求头
    fn build_headers(access_token: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&format!("Bearer {}", access_token)).unwrap(),
        );
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        headers.insert(
            "User-Agent",
            HeaderValue::from_static("google-cloud-sdk gcloud/0.0.0.dev"),
        );
        headers.insert(
            "X-Goog-Api-Client",
            HeaderValue::from_static("gl-python/3.12.0"),
        );
        headers.insert(
            "Client-Metadata",
            HeaderValue::from_static(
                r#"{"ideType":"IDE_UNSPECIFIED","platform":"PLATFORM_UNSPECIFIED","pluginType":"GEMINI"}"#,
            ),
        );
        headers
    }

    /// 解析响应内容
    fn parse_response_content(text: &str) -> String {
        let v: serde_json::Value = match serde_json::from_str(text) {
            Ok(v) => v,
            Err(_) => return "[无法解析响应]".to_string(),
        };

        // 两种格式：
        // Format 1: { "response": { "candidates": [...] } }
        // Format 2: { "candidates": [...] }
        let candidates = v
            .get("response")
            .and_then(|r| r.get("candidates"))
            .or_else(|| v.get("candidates"));

        candidates
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("content"))
            .and_then(|c| c.get("parts"))
            .and_then(|p| p.get(0))
            .and_then(|p| p.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("[无法解析响应]")
            .to_string()
    }
}

#[async_trait]
impl LlmProvider for AntigravityProvider {
    fn id(&self) -> &str {
        "antigravity"
    }

    fn auth(&self) -> &Auth {
        &self.auth_enum
    }

    fn supported_models(&self) -> &[&str] {
        &["gemini-2.5-flash", "gemini-2.5-pro", "gemini-1.5-pro"]
    }

    fn compile(&self, primitive: &PrimitiveRequest) -> serde_json::Value {
        let mut req = primitive.clone();
        if req.model.is_empty() {
            req.model = self.config.model.clone();
        }

        let compiler = crate::provider::gemini::compiler::GeminiCompiler;
        let inner_request = compiler.compile(&req);
        
        let mut request_payload = serde_json::json!({
            "sessionId": format!("-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() & 0x7FFFFFFFFFFFFFFF)
        });
        
        if let Some(contents) = inner_request.get("contents") {
            request_payload["contents"] = contents.clone();
        } else {
            request_payload["contents"] = serde_json::json!([]);
        }
        
        if let Some(sys) = inner_request.get("systemInstruction") {
            if !sys.is_null() {
                request_payload["systemInstruction"] = sys.clone();
            }
        }

        serde_json::json!({
            "model": req.model,
            "userAgent": "antigravity",
            "requestType": "agent",
            "project": "", // 会在 complete/stream 动态注入
            "requestId": format!("agent-{}", uuid::Uuid::new_v4()),
            "request": request_payload
        })
    }

    async fn complete(&self, mut body: serde_json::Value) -> crate::Result<LlmResponse> {
        let access_token = self.get_access_token().await?;
        let project_id = self.get_project_id().await?;

        let project = match project_id.as_deref() {
            Some(pid) if !pid.is_empty() => pid.to_string(),
            _ => Self::generate_project_id(),
        };

        if let Some(obj) = body.as_object_mut() {
            obj.insert("project".to_string(), serde_json::Value::String(project));
        }
        let request_body = body;

        let url = format!("{}/{}:generateContent", BASE_URL, API_VERSION);

        let resp = self
            .http
            .post(&url)
            .headers(Self::build_headers(&access_token))
            .json(&request_body)
            .send()
            .await
            .map_err(|e| crate::Error::Http(e.to_string()))?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(crate::Error::Provider(format!(
                "Antigravity API failed ({}): {}",
                status,
                text.trim()
            )));
        }

        let content = Self::parse_response_content(&text);

        Ok(LlmResponse {
            content,
            tool_calls: Vec::new(),
            usage: Usage::default(),
            stop_reason: StopReason::EndTurn,
        })
    }

    async fn stream(
        &self,
        mut body: serde_json::Value,
    ) -> crate::Result<BoxStream<'_, crate::Result<LlmChunk>>> {
        let access_token = self.get_access_token().await?;
        let project_id = self.get_project_id().await?;

        let project = match project_id.as_deref() {
            Some(pid) if !pid.is_empty() => pid.to_string(),
            _ => Self::generate_project_id(),
        };

        if let Some(obj) = body.as_object_mut() {
            obj.insert("project".to_string(), serde_json::Value::String(project));
        }
        let request_body = body;

        let url = format!(
            "{}/{}:streamGenerateContent?alt=sse",
            BASE_URL, API_VERSION
        );

        let resp = self
            .http
            .post(&url)
            .headers(Self::build_headers(&access_token))
            .json(&request_body)
            .send()
            .await
            .map_err(|e| crate::Error::Http(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(crate::Error::Provider(format!(
                "Antigravity stream failed ({}): {}",
                status,
                text.trim()
            )));
        }

        use futures::StreamExt;

        let stream = async_stream::stream! {
            let mut byte_stream = resp.bytes_stream();
            let mut buffer = String::new();

            while let Some(chunk) = byte_stream.next().await {
                let bytes = match chunk {
                    Ok(b) => b,
                    Err(e) => {
                        yield Err(crate::Error::Http(e.to_string()));
                        continue;
                    }
                };
                buffer.push_str(&String::from_utf8_lossy(&bytes));

                while let Some(pos) = buffer.find("\r\n\r\n").or_else(|| buffer.find("\n\n")) {
                    let offset = if buffer[pos..].starts_with("\r\n\r\n") { 4 } else { 2 };
                    let event = buffer[..pos].to_string();
                    buffer = buffer[pos + offset..].to_string();

                    for line in event.lines() {
                        if let Some(data) = line.strip_prefix("data: ") {
                            if data == "[DONE]" {
                                return;
                            }
                            if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
                                let candidates = v
                                    .get("response")
                                    .and_then(|r| r.get("candidates"))
                                    .or_else(|| v.get("candidates"));

                                if let Some(text) = candidates
                                    .and_then(|c| c.get(0).or_else(|| c.as_array().and_then(|a| a.get(0))))
                                    .and_then(|c| c.get("content"))
                                    .and_then(|c| c.get("parts"))
                                    .and_then(|p| p.get(0).or_else(|| p.as_array().and_then(|a| a.get(0))))
                                    .and_then(|p| p.get("text"))
                                    .and_then(|t| t.as_str())
                                {
                                    if !text.is_empty() {
                                        yield Ok(LlmChunk {
                                            delta: crate::provider::ChunkDelta::Text(text.to_string()),
                                            usage: None,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }

    fn needs_refresh(&self) -> bool {
        // 尝试非阻塞获取锁检查状态
        if let Ok(auth_guard) = self.auth.try_lock() {
            auth_guard.needs_refresh()
        } else {
            false // 无法获取锁，假设不需要刷新
        }
    }

    async fn refresh_auth(&mut self) -> crate::Result<()> {
        let mut auth_guard = self.auth.lock().await;
        auth_guard
            .ensure_authenticated()
            .await
            .map_err(|e| crate::Error::Auth(e.to_string()))
    }
}
