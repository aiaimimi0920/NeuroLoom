use super::config::IFlowConfig;
use crate::auth::Auth;
use crate::primitive::PrimitiveRequest;
use crate::provider::{LlmProvider, LlmResponse, BoxStream, LlmChunk, StopReason, Usage};
use async_trait::async_trait;
use serde::Deserialize;
use reqwest::header::{HeaderMap, HeaderValue};
use std::time::Duration;

pub struct IFlowProvider {
    #[allow(dead_code)]
    config: IFlowConfig,
    auth_enum: Auth,
}

impl IFlowProvider {
    pub fn new(config: IFlowConfig) -> Self {
        // For iFlow, we need cookie-based auth but the trait requires Auth enum.
        // We use a placeholder ApiKey auth since there's no Cookie variant.
        let auth_enum = Auth::ApiKey(crate::auth::ApiKeyConfig::new(
            config.cookie.clone(),
            crate::auth::ApiKeyProvider::IFlow,
        ));
        Self { config, auth_enum }
    }

    /// 构建包含 Auth Cookie 和防风控的浏览器请求头
    fn build_headers(&self, is_post: bool) -> crate::Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Cookie",
            HeaderValue::from_str(&self.config.cookie).map_err(|e| crate::Error::Provider(e.to_string()))?,
        );
        headers.insert("Accept", HeaderValue::from_static("application/json, text/plain, */*"));
        headers.insert(
            "User-Agent",
            HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/120.0.0.0 Safari/537.36"),
        );
        headers.insert("Sec-Fetch-Dest", HeaderValue::from_static("empty"));
        headers.insert("Sec-Fetch-Mode", HeaderValue::from_static("cors"));
        headers.insert("Sec-Fetch-Site", HeaderValue::from_static("same-origin"));

        if is_post {
            headers.insert("Content-Type", HeaderValue::from_static("application/json"));
            headers.insert("Origin", HeaderValue::from_static("https://platform.iflow.cn"));
            headers.insert("Referer", HeaderValue::from_static("https://platform.iflow.cn/"));
        }
        Ok(headers)
    }

    /// 通过 Cookie 获取 API Key (iFlow 特有)
    pub async fn fetch_api_key(&self) -> crate::Result<String> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| crate::Error::Provider(e.to_string()))?;

        // 1. GET获取基础信息
        let get_resp = client
            .get("https://platform.iflow.cn/api/openapi/apikey")
            .headers(self.build_headers(false)?)
            .send()
            .await
            .map_err(|e| crate::Error::Provider(e.to_string()))?;

        if !get_resp.status().is_success() {
            return Err(crate::Error::Provider(format!("GET status {}", get_resp.status())));
        }

        let json_get: IFlowApiKeyResponse = get_resp.json().await.map_err(|e| crate::Error::Provider(e.to_string()))?;
        if !json_get.success {
            return Err(crate::Error::Provider("GET response success=false".to_string()));
        }

        let data = json_get.data.ok_or_else(|| crate::Error::Provider("Missing data in GET".to_string()))?;

        // 如果未过期，并且我们有被掩码的或者完整的api key，返回即可（如果需要完整的通常必须POST刷新，但样例中只要有就行）
        // 但安全起见，我们总是进行一步POST刷新来获取真实的完整Token
        
        // 2. POST 刷新 API Key
        let post_body = serde_json::json!({
            "name": data.name
        });

        let post_resp = client
            .post("https://platform.iflow.cn/api/openapi/apikey")
            .headers(self.build_headers(true)?)
            .json(&post_body)
            .send()
            .await
            .map_err(|e| crate::Error::Provider(e.to_string()))?;

        if !post_resp.status().is_success() {
            return Err(crate::Error::Provider(format!("POST status {}", post_resp.status())));
        }

        let json_post: IFlowApiKeyResponse = post_resp.json().await.map_err(|e| crate::Error::Provider(e.to_string()))?;
        if !json_post.success {
            return Err(crate::Error::Provider("POST response success=false".to_string()));
        }

        let post_data = json_post.data.ok_or_else(|| crate::Error::Provider("Missing data in POST".to_string()))?;
        Ok(post_data.api_key)
    }
}

// 内部JSON响应结构
#[derive(Debug, Deserialize)]
struct IFlowApiKeyResponse {
    #[serde(default)]
    success: bool,
    #[serde(default)]
    data: Option<IFlowKeyData>,
}

#[derive(Debug, Deserialize)]
struct IFlowKeyData {
    name: String,
    #[serde(rename = "apiKey", default)]
    api_key: String,
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
        &["qwen3-max"]
    }

    fn compile(&self, primitive: &PrimitiveRequest) -> serde_json::Value {
        // IFlow can reuse openai format with minor tweaks
        let mut req = primitive.clone();
        if req.model.is_empty() {
            req.model = self.config.model.clone();
        }
        
        let mut body = crate::translator::wrapper::openai::wrap(&req).unwrap_or_default();

        // 注入 Thinking (Reasoning) 参数
        let model = self.config.model.to_lowercase();
        let is_thinking = model.starts_with("glm") 
            || matches!(model.as_str(), "qwen3-max-preview" | "deepseek-v3.2" | "deepseek-v3.1" | "deepseek-r1");
            
        if is_thinking {
            if let Some(obj) = body.as_object_mut() {
                let kwargs = obj.entry("chat_template_kwargs").or_insert(serde_json::json!({}));
                if let Some(kwargs_obj) = kwargs.as_object_mut() {
                    kwargs_obj.insert("enable_thinking".to_string(), serde_json::Value::Bool(true));
                    if model.starts_with("glm") {
                         kwargs_obj.insert("clear_thinking".to_string(), serde_json::Value::Bool(false));
                    }
                }
            }
        } else if model.starts_with("minimax") {
            if let Some(obj) = body.as_object_mut() {
                 obj.insert("reasoning_split".to_string(), serde_json::Value::Bool(true));
            }
        }

        body
    }

    async fn complete(&self, body: serde_json::Value) -> crate::Result<LlmResponse> {
        let api_key = self.fetch_api_key().await?;
        
        let client = reqwest::Client::new();
        let resp = client
            .post("https://apis.iflow.cn/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/120.0.0.0 Safari/537.36")
            .json(&body)
            .send()
            .await
            .map_err(|e| crate::Error::Provider(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(crate::Error::Provider(format!("iflow chat failed: [{}] {}", status, text)));
        }

        let raw_text = resp.text().await.unwrap_or_default();
        let json_resp: serde_json::Value = serde_json::from_str(&raw_text).map_err(|e| crate::Error::Provider(e.to_string()))?;
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

    async fn stream(&self, mut body: serde_json::Value) -> crate::Result<BoxStream<'_, crate::Result<LlmChunk>>> {
        body["stream"] = serde_json::Value::Bool(true);
        let api_key = self.fetch_api_key().await?;

        let client = reqwest::Client::new();
        let resp = client
            .post("https://apis.iflow.cn/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/120.0.0.0 Safari/537.36")
            .json(&body)
            .send()
            .await
            .map_err(|e| crate::Error::Provider(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(crate::Error::Provider(format!("iflow chat stream failed: [{}] {}", status, text)));
        }

        use futures::StreamExt;
        
        let stream = async_stream::stream! {
            let mut byte_stream = resp.bytes_stream();
            let mut buffer = String::new();

            while let Some(chunk_res) = byte_stream.next().await {
                let bytes = chunk_res.map_err(|e| crate::Error::Provider(e.to_string()))?;
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
                                            delta: crate::provider::ChunkDelta::Text(content.to_string()),
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
