use serde_json::{json, Value};
use tokio_stream::StreamExt;

use crate::primitive::{PrimitiveMessage, PrimitiveRequest};
use crate::protocol::error::{ErrorKind, StandardError};
use crate::protocol::traits::ProtocolFormat;
use crate::provider::{BoxLlmStream, LlmChunk, LlmResponse};

/// 腾讯云 TI 平台大模型 API V3 协议封包与解包
/// Tencent Cloud API V3 (TC3) 对于模型调用的参数格式与标准 OpenAI 有差异：
/// 1. 字段名称使用 PascalCase（如 Model, Messages, Stream）
/// 2. 响应体（非流式）通常被包裹在 `{"Response": {...}}` 中
/// 3. Message 本身字段也是 PascalCase（如 Role, Content）
/// 4. 响应的选项卡为 Choices[0].Messages 或 Choices[0].Message
pub struct TencentTiProtocol;

impl ProtocolFormat for TencentTiProtocol {
    fn id(&self) -> &str {
        "tencent_ti"
    }

    fn pack(&self, primitive: &PrimitiveRequest, is_stream: bool) -> Value {
        let mut messages = Vec::new();

        // 将 system 映射为第一条 Role: "system" 的消息
        if let Some(sys) = &primitive.system {
            messages.push(json!({
                "Role": "system",
                "Content": sys
            }));
        }

        // 转换其余消息
        for msg in &primitive.messages {
            messages.push(Self::pack_message(msg));
        }

        let mut body = json!({
            "Model": primitive.model,
            "Messages": messages,
        });

        if is_stream {
            body["Stream"] = json!(true);
            body["StreamOptions"] = json!({"IncludeUsage": true});
        }

        // Apply parameters
        let params = &primitive.parameters;
        if let Some(temp) = params.temperature {
            body["Temperature"] = json!(temp);
        }
        if let Some(top_p) = params.top_p {
            body["TopP"] = json!(top_p);
        }

        // 推理服务和特定功能可能有额外的专属参数，例如 enable_enhancement 等
        if let Some(effort) = primitive.extra.get("enable_enhancement") {
            body["EnableEnhancement"] = effort.clone();
        }

        body
    }

    fn unpack_response(&self, raw: &str) -> anyhow::Result<LlmResponse> {
        let v: Value = serde_json::from_str(raw)
            .map_err(|e| anyhow::anyhow!("Failed to parse JSON: {}", e))?;

        // 腾讯云 API V3 的标准 HTTP API 响应结果都会被包裹在 "Response" 字段下。
        let resp = v.get("Response").unwrap_or(&v);

        // 如果存在 Error
        if let Some(error) = resp.get("Error") {
            let msg = error.get("Message").and_then(|m| m.as_str()).unwrap_or(raw);
            return Err(anyhow::anyhow!("Tencent API Error: {}", msg));
        }

        let choices = resp.get("Choices").or_else(|| resp.get("choices")).and_then(|c| c.as_array());
        
        // 尝试获取 Choices[0].Message.Content 或 Choices[0].Messages.Content (兼容不同的字段名拼写返回)
        let content = if let Some(choice) = choices.and_then(|a| a.first()) {
            choice.get("Message").or_else(|| choice.get("Messages"))
                .and_then(|m| m.get("Content").or_else(|| m.get("content")))
                .and_then(|s| s.as_str())
                .unwrap_or_default()
                .to_string()
        } else {
            String::new()
        };

        // 获取 Usage
        let usage = resp.get("Usage").or_else(|| resp.get("usage")).map(|u| crate::provider::Usage {
            prompt_tokens: u.get("PromptTokens").or_else(|| u.get("prompt_tokens")).and_then(|t| t.as_u64()).unwrap_or(0) as u32,
            completion_tokens: u.get("CompletionTokens").or_else(|| u.get("completion_tokens")).and_then(|t| t.as_u64()).unwrap_or(0) as u32,
            total_tokens: u.get("TotalTokens").or_else(|| u.get("total_tokens")).and_then(|t| t.as_u64()).unwrap_or(0) as u32,
        });

        // 获取 Model
        let model = resp.get("Model").or_else(|| v.get("Model"))
            .and_then(|m| m.as_str())
            .unwrap_or("unknown")
            .to_string();

        Ok(LlmResponse {
            content,
            model,
            usage,
        })
    }

    fn unpack_stream(&self, resp: reqwest::Response) -> anyhow::Result<BoxLlmStream> {
        let stream = async_stream::stream! {
            let mut byte_stream = resp.bytes_stream();
            let mut buffer = String::new();

            while let Some(chunk_result) = byte_stream.next().await {
                let bytes = match chunk_result {
                    Ok(b) => b,
                    Err(e) => {
                        yield Err(anyhow::anyhow!("Http error during stream: {}", e));
                        continue;
                    }
                };

                let s = String::from_utf8_lossy(&bytes);
                buffer.push_str(&s);

                while let Some(pos) = buffer.find('\n') {
                    let line = buffer[..pos].trim().to_string();
                    buffer = buffer[pos + 1..].to_string();

                    // 处理 SSE 事件数据，如 "data: {...}"
                    let data_str = line.strip_prefix("data: ")
                        .or_else(|| line.strip_prefix("data:"));

                    if let Some(data) = data_str {
                        let data = data.trim();
                        if data == "[DONE]" || data.is_empty() {
                            continue;
                        }

                        if let Ok(json) = serde_json::from_str::<Value>(data) {
                            // 流式通常直接放在块的根部，或者依然具有 Response 包装
                            let resp = json.get("Response").unwrap_or(&json);
                            
                            if let Some(choices) = resp.get("Choices").or_else(|| resp.get("choices")).and_then(|c| c.as_array()) {
                                if !choices.is_empty() {
                                    if let Some(delta_content) = choices[0].get("Delta").or_else(|| choices[0].get("delta")).and_then(|d| d.get("Content").or_else(|| d.get("content"))).and_then(|c| c.as_str()) {
                                        if !delta_content.is_empty() {
                                            yield Ok(LlmChunk {
                                                content: delta_content.to_string(),
                                            });
                                        }
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

    fn matches_format(&self, data: &Value) -> bool {
        data.get("Model").is_some() || data.get("Messages").is_some() || (data.get("Response").is_some() && data["Response"].get("Choices").is_some())
    }

    fn unpack_error(&self, status: u16, raw: &str) -> anyhow::Result<StandardError> {
        let json: Value = serde_json::from_str(raw).unwrap_or_else(|_| json!({}));
        let resp = json.get("Response").unwrap_or(&json);
        let error = resp.get("Error").unwrap_or(resp);

        let kind = match status {
            401 | 403 => ErrorKind::Authentication,
            429 => ErrorKind::RateLimit,
            500..=599 => ErrorKind::ServerError,
            _ => ErrorKind::Other,
        };

        let message = error
            .get("Message")
            .or_else(|| error.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or(raw)
            .to_string();

        let code = error
            .get("Code")
            .or_else(|| error.get("code"))
            .and_then(|c| c.as_str())
            .map(|s| s.to_string());

        Ok(StandardError {
            kind,
            message,
            code,
            retryable: matches!(kind, ErrorKind::RateLimit | ErrorKind::ServerError),
            fallback_hint: None,
        })
    }
}

impl TencentTiProtocol {
    fn pack_message(msg: &PrimitiveMessage) -> Value {
        let role = msg.role.to_string();
        let content_str = msg
            .content
            .iter()
            .filter_map(|c| {
                if let crate::primitive::message::PrimitiveContent::Text { text } = c {
                    Some(text.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        json!({
            "Role": role,
            "Content": content_str
        })
    }
}
