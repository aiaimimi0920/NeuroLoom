//! IFlow Provider 实现
//!
//! 使用 OpenAI 兼容协议，通过 Cookie 认证获取 API Key

use super::config::IFlowConfig;
use crate::auth::providers::iflow::IFlowAuth;
use crate::auth::Auth;
use crate::primitive::PrimitiveRequest;
use crate::provider::{BoxStream, LlmChunk, LlmResponse, StopReason, Usage, GenericClient, Endpoint, Protocol};
use crate::generic_client;
use async_trait::async_trait;
use tokio::sync::Mutex;
use std::sync::Arc;

/// iFlow 支持的模型常量
pub mod models {
    /// Thinking 模型列表 (支持推理输出)
    pub const THINKING_MODELS: &[&str] = &[
        "glm-4-plus",
        "glm-4-air",
        "glm-4-airx",
        "glm-4-flash",
        "glm-4-long",
        "qwen3-max-preview",
        "deepseek-v3.2",
        "deepseek-v3.1",
        "deepseek-r1",
    ];

    /// 需要 reasoning_split 的模型
    pub const REASONING_SPLIT_MODELS: &[&str] = &["minimax-01"];

    /// 默认模型
    pub const DEFAULT_MODEL: &str = "qwen3-max";
}

// ── Orthogonal Decomposition: Protocol & Endpoint ─────────────────────────────

pub struct IFlowProtocol {
    pub default_model: String,
}

impl Protocol for IFlowProtocol {
    fn compile(&self, primitive: &PrimitiveRequest) -> serde_json::Value {
        let mut req = primitive.clone();
        if req.model.is_empty() {
            req.model = self.default_model.clone();
        }

        let mut body = crate::translator::wrapper::openai::wrap(&req).unwrap_or_default();

        // 注入 Thinking (Reasoning) 参数
        let model = req.model.to_lowercase();

        let is_thinking = models::THINKING_MODELS
            .iter()
            .any(|m| model.starts_with(&m.to_lowercase()))
            || model.starts_with("glm");

        let needs_reasoning_split = models::REASONING_SPLIT_MODELS
            .iter()
            .any(|m| model.starts_with(&m.to_lowercase()));

        if is_thinking {
            if let Some(obj) = body.as_object_mut() {
                let kwargs = obj
                    .entry("chat_template_kwargs")
                    .or_insert(serde_json::json!({}));
                if let Some(kwargs_obj) = kwargs.as_object_mut() {
                    kwargs_obj.insert("enable_thinking".to_string(), serde_json::Value::Bool(true));
                    if model.starts_with("glm") {
                        kwargs_obj
                            .insert("clear_thinking".to_string(), serde_json::Value::Bool(false));
                    }
                }
            }
        } else if needs_reasoning_split {
            if let Some(obj) = body.as_object_mut() {
                obj.insert("reasoning_split".to_string(), serde_json::Value::Bool(true));
            }
        }

        body
    }

    fn parse_response(&self, raw_text: &str) -> crate::Result<LlmResponse> {
        let json_resp: serde_json::Value =
            serde_json::from_str(&raw_text).map_err(|e| crate::Error::Json(e))?;

        let content = json_resp["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .to_string();

        Ok(LlmResponse {
            content,
            tool_calls: Vec::new(),
            usage: Usage::default(),
            stop_reason: StopReason::EndTurn,
        })
    }

    fn parse_stream(
        &self,
        resp: reqwest::Response,
    ) -> crate::Result<BoxStream<'static, crate::Result<LlmChunk>>> {
        use futures::StreamExt;

        let stream = async_stream::stream! {
            let mut byte_stream = resp.bytes_stream();
            let mut buffer = String::new();

            while let Some(chunk_res) = byte_stream.next().await {
                let bytes = match chunk_res {
                    Ok(b) => b,
                    Err(e) => {
                        yield Err(crate::Error::Http(e.to_string()));
                        continue;
                    }
                };
                let s = String::from_utf8_lossy(&bytes);
                buffer.push_str(&s);

                // Check if this might be a pure non-SSE JSON response returned by mistake
                if buffer.starts_with('{') && buffer.ends_with('}') {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&buffer) {
                        if let Some(content) = v["choices"][0]["message"]["content"].as_str() {
                            yield Ok(LlmChunk {
                                delta: crate::provider::ChunkDelta::Text(content.to_string()),
                                usage: None,
                            });
                            buffer.clear();
                            return;
                        }
                    }
                }

                while let Some(pos) = buffer.find("\n\n").or_else(|| buffer.find("\r\n\r\n")) {
                    let offset = if buffer[pos..].starts_with("\r\n\r\n") { 4 } else { 2 };
                    let event = buffer[..pos].to_string();
                    buffer = buffer[pos + offset..].to_string();

                    for line in event.lines() {
                        if let Some(raw_data) = line.strip_prefix("data:") {
                            let data = raw_data.trim_start();
                            if data == "[DONE]" {
                                return;
                            }
                            if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
                                if let Some(content) = v["choices"][0]["delta"]["content"].as_str() {
                                    if !content.is_empty() {
                                        yield Ok(LlmChunk {
                                            delta: crate::provider::ChunkDelta::Text(
                                                content.to_string(),
                                            ),
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
}

pub struct IFlowEndpoint {
    auth: Arc<Mutex<IFlowAuth>>,
}

#[async_trait]
impl Endpoint for IFlowEndpoint {
    async fn pre_flight(&self) -> crate::Result<()> {
        let mut auth_guard = self.auth.lock().await;
        auth_guard.ensure_authenticated().await.map_err(|e| crate::Error::Auth(e.to_string()))?;
        Ok(())
    }

    fn url(&self, _model: &str, _is_stream: bool) -> crate::Result<String> {
        Ok("https://apis.iflow.cn/v1/chat/completions".to_string())
    }

    fn decorate_body(&self, mut body: serde_json::Value) -> serde_json::Value {
        // Stream 选项需要在此注入，因为泛型在 stream 请求里发的是同一套体
        if body.get("stream").is_none() {
            body["stream"] = serde_json::Value::Bool(false);
        }
        body
    }

    fn inject_auth(&self, req: reqwest::RequestBuilder) -> crate::Result<reqwest::RequestBuilder> {
        // 获取预先解密的 token，一定能在 pre_flight() 后的缓存拿到
        let api_key = {
            let guard = self.auth.try_lock().map_err(|_| crate::Error::Auth("Auth Mutex Failed".to_string()))?;
            guard.api_key().map(|s| s.to_string()).ok_or_else(|| crate::Error::Auth("No API key available".to_string()))?
        };

        let req = req
            .header("Authorization", format!("Bearer {}", api_key))
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            );
        Ok(req)
    }

    fn needs_refresh(&self) -> bool {
        if let Ok(guard) = self.auth.try_lock() {
            guard.needs_refresh()
        } else {
            false
        }
    }

    async fn refresh_auth(&self) -> crate::Result<()> {
        let mut auth_guard = self.auth.lock().await;
        auth_guard.ensure_authenticated().await.map_err(|e| crate::Error::Auth(e.to_string()))?;
        Ok(())
    }
}

// ── GenericClient alias IFlowProvider ───────────────────────────────────────

pub type IFlowProvider = GenericClient<IFlowEndpoint, IFlowProtocol>;

impl IFlowProvider {
    /// 创建新的 IFlow Provider（需要外部传入 HTTP Client）
    ///
    /// 注意：根据设计规范，HTTP Client 应由外部统一管理，
    /// 避免每个 Provider 重复创建连接池。
    pub fn new(config: IFlowConfig, http: reqwest::Client) -> Self {
        let mut auth = IFlowAuth::from_file(&config.token_path).unwrap_or_else(|_| IFlowAuth::new());
        auth.set_cookie(&config.cookie);

        let auth_enum = Auth::IFlowCookie {
            cookie: config.cookie.clone(),
            token_path: config.token_path.clone(),
        };

        let shared_auth = Arc::new(Mutex::new(auth));

        generic_client! {
            id: "iflow".to_string(),
            endpoint: IFlowEndpoint { auth: shared_auth.clone() },
            protocol: IFlowProtocol { default_model: config.model.clone() },
            auth: auth_enum,
            supported_models: vec![
                "qwen3-max".to_string(),
                "qwen3-max-preview".to_string(),
                "deepseek-v3.2".to_string(),
                "glm-4-plus".to_string()
            ],
            http: http
        }
    }

    /// 获取 API Key（公开方法，供示例和调试使用）
    pub async fn fetch_api_key(&self) -> crate::Result<String> {
        let mut auth_guard = self.endpoint.auth.lock().await;
        auth_guard.fetch_api_key().await.map_err(|e| crate::Error::Auth(e.to_string()))
    }

    /// 清除 API Key 缓存
    pub async fn clear_cache(&self) {
        let mut auth_guard = self.endpoint.auth.lock().await;
        auth_guard.clear_cache();
    }
}
