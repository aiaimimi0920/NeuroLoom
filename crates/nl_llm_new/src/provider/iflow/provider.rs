//! IFlow Provider 实现
//!
//! 使用 OpenAI 兼容协议，通过 Cookie 认证获取 API Key

use super::config::IFlowConfig;
use crate::auth::providers::iflow::IFlowAuth;
use crate::auth::Auth;
use crate::primitive::PrimitiveRequest;
use crate::provider::{BoxStream, LlmChunk, LlmProvider, LlmResponse, StopReason, Usage};
use async_trait::async_trait;
use std::time::Duration;
use tokio::sync::Mutex;

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

/// IFlow Provider
///
/// 使用 Cookie 认证，通过 iFlow 平台获取 API Key 后调用 OpenAI 兼容接口
pub struct IFlowProvider {
    config: IFlowConfig,
    /// 认证实例（使用 tokio::Mutex 支持异步跨 await）
    auth: Mutex<IFlowAuth>,
    /// 复用的 HTTP Client（用于 API 调用）
    http: reqwest::Client,
    /// Auth enum 用于 trait 方法返回
    auth_enum: Auth,
}

impl IFlowProvider {
    /// 创建新的 IFlow Provider
    pub fn new(config: IFlowConfig) -> Self {
        let auth = IFlowAuth::new(config.cookie.clone());
        let auth_enum = Auth::ApiKey(crate::auth::ApiKeyConfig::new(
            config.cookie.clone(),
            crate::auth::ApiKeyProvider::IFlow,
        ));

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

    /// 获取 API Key（从��存或新获取）
    async fn get_api_key(&self) -> crate::Result<String> {
        let mut auth_guard = self.auth.lock().await;
        let key = auth_guard.get_api_key().await.map_err(|e| crate::Error::Auth(e.to_string()))?;
        Ok(key.to_string())
    }

    /// 清除 API Key 缓存
    pub async fn clear_cache(&self) {
        let mut auth_guard = self.auth.lock().await;
        auth_guard.clear_cache();
    }

    /// 获取 API Key（公开方法，供示例和调试使用）
    ///
    /// 强制刷新并返回新的 API Key
    pub async fn fetch_api_key(&self) -> crate::Result<String> {
        let mut auth_guard = self.auth.lock().await;
        let key = auth_guard
            .fetch_api_key()
            .await
            .map_err(|e| crate::Error::Auth(e.to_string()))?;
        Ok(key.to_string())
    }

    /// 检查模型是否支持 Thinking 模式
    fn is_thinking_model(model: &str) -> bool {
        let model_lower = model.to_lowercase();
        models::THINKING_MODELS
            .iter()
            .any(|m| model_lower.starts_with(&m.to_lowercase()))
            || model_lower.starts_with("glm")
    }

    /// 检查模型是否需要 reasoning_split
    fn needs_reasoning_split(model: &str) -> bool {
        let model_lower = model.to_lowercase();
        models::REASONING_SPLIT_MODELS
            .iter()
            .any(|m| model_lower.starts_with(&m.to_lowercase()))
    }
}

#[async_trait]
impl LlmProvider for IFlowProvider {
    fn id(&self) -> &str {
        "iflow"
    }

    fn auth(&self) -> &Auth {
        &self.auth_enum
    }

    fn supported_models(&self) -> &[&str] {
        &["qwen3-max", "qwen3-max-preview", "deepseek-v3.2", "glm-4-plus"]
    }

    fn compile(&self, primitive: &PrimitiveRequest) -> serde_json::Value {
        let mut req = primitive.clone();
        if req.model.is_empty() {
            req.model = self.config.model.clone();
        }

        let mut body = crate::translator::wrapper::openai::wrap(&req).unwrap_or_default();

        // 注入 Thinking (Reasoning) 参数
        let model = self.config.model.to_lowercase();

        if Self::is_thinking_model(&model) {
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
        } else if Self::needs_reasoning_split(&model) {
            if let Some(obj) = body.as_object_mut() {
                obj.insert("reasoning_split".to_string(), serde_json::Value::Bool(true));
            }
        }

        body
    }

    async fn complete(&self, body: serde_json::Value) -> crate::Result<LlmResponse> {
        let api_key = self.get_api_key().await?;

        let resp = self
            .http
            .post("https://apis.iflow.cn/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .json(&body)
            .send()
            .await
            .map_err(|e| crate::Error::Http(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(crate::Error::Provider(format!(
                "iflow chat failed: [{}] {}",
                status, text
            )));
        }

        let raw_text = resp.text().await.unwrap_or_default();
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

    async fn stream(
        &self,
        mut body: serde_json::Value,
    ) -> crate::Result<BoxStream<'_, crate::Result<LlmChunk>>> {
        body["stream"] = serde_json::Value::Bool(true);
        let api_key = self.get_api_key().await?;

        let resp = self
            .http
            .post("https://apis.iflow.cn/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .json(&body)
            .send()
            .await
            .map_err(|e| crate::Error::Http(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(crate::Error::Provider(format!(
                "iflow chat stream failed: [{}] {}",
                status, text
            )));
        }

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

    fn needs_refresh(&self) -> bool {
        // IFlow API Key 无明确过期时间，不需要自动刷新
        false
    }

    async fn refresh_auth(&mut self) -> crate::Result<()> {
        // 清除缓存，下次请求时会重新获取
        self.clear_cache().await;
        Ok(())
    }
}
