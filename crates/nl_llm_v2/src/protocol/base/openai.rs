use serde_json::{json, Value};
use tokio_stream::StreamExt;

use crate::protocol::traits::ProtocolFormat;
use crate::protocol::error::{StandardError, ErrorKind};
use crate::primitive::{PrimitiveRequest, PrimitiveMessage};
use crate::provider::{LlmResponse, BoxLlmStream, LlmChunk};

/// OpenAI 标准协议封包与解包
pub struct OpenAiProtocol;

impl ProtocolFormat for OpenAiProtocol {
    fn id(&self) -> &str {
        "openai"
    }

    fn pack(&self, primitive: &PrimitiveRequest, is_stream: bool) -> Value {
        let mut body = json!({
            "model": primitive.model,
            "messages": primitive.messages.iter().map(Self::pack_message).collect::<Vec<_>>(),
        });

        if is_stream {
            body["stream"] = json!(true);
        }

        if let Some(sys) = &primitive.system {
            if let Some(msgs) = body["messages"].as_array_mut() {
                msgs.insert(0, json!({
                    "role": "system",
                    "content": sys
                }));
            }
        }

        // Apply parameters
        let params = &primitive.parameters;
        if let Some(temp) = params.temperature {
            body["temperature"] = json!(temp);
        }
        if let Some(top_p) = params.top_p {
            body["top_p"] = json!(top_p);
        }
        if let Some(max_tok) = params.max_tokens {
            body["max_tokens"] = json!(max_tok);
        }

        body
    }

    fn unpack_response(&self, raw: &str) -> anyhow::Result<LlmResponse> {
        let v: Value = serde_json::from_str(raw)
            .map_err(|e| anyhow::anyhow!("Failed to parse JSON: {}", e))?;

        // [修复] 正确解析 content
        let content = v["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .to_string();

        // [修复] 正确解析 model 字段
        let model = v.get("model")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown")
            .to_string();

        // [修复] 正确解析 usage
        let usage = v.get("usage").map(|u| {
            crate::provider::Usage {
                prompt_tokens: u.get("prompt_tokens").and_then(|t| t.as_u64()).unwrap_or(0) as u32,
                completion_tokens: u.get("completion_tokens").and_then(|t| t.as_u64()).unwrap_or(0) as u32,
                total_tokens: u.get("total_tokens").and_then(|t| t.as_u64()).unwrap_or(0) as u32,
            }
        });

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

                    // 兼容 "data: {...}" 和 "data:{...}" 两种 SSE 格式
                    // 标准 SSE 使用 "data: " (带空格), 但部分服务（如 iFlow）省略空格
                    let data_str = line.strip_prefix("data: ")
                        .or_else(|| line.strip_prefix("data:"));
                    if let Some(data) = data_str {
                        let data = data.trim();
                        if data == "[DONE]" || data.is_empty() {
                            continue;
                        }
                        
                        if let Ok(json) = serde_json::from_str::<Value>(data) {
                            if let Some(choices) = json.get("choices").and_then(|c| c.as_array()) {
                                if !choices.is_empty() {
                                    if let Some(delta_content) = choices[0].get("delta").and_then(|d| d.get("content")).and_then(|c| c.as_str()) {
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
        data.get("messages").is_some() && data.get("model").is_some()
    }

    fn unpack_error(&self, status: u16, raw: &str) -> anyhow::Result<StandardError> {
        // [修复] 解析 OpenAI 错误 JSON 获取详细信息
        // 原因：错误需要包含具体 message 和 code，便于调试
        let json: Value = serde_json::from_str(raw).unwrap_or_else(|_| json!({}));
        let error = &json["error"];

        let kind = match status {
            401 | 403 => ErrorKind::Authentication,
            429 => ErrorKind::RateLimit,
            500..=599 => ErrorKind::ServerError,
            _ => match error.get("type").and_then(|t| t.as_str()) {
                Some("context_length_exceeded") => ErrorKind::ContextLengthExceeded,
                Some("content_filter") => ErrorKind::ContentFilter,
                Some("model_not_found") | Some("model_not_available") => ErrorKind::ModelUnavailable,
                _ => ErrorKind::Other,
            }
        };

        let message = error.get("message")
            .and_then(|m| m.as_str())
            .unwrap_or(raw)
            .to_string();

        let code = error.get("code")
            .or_else(|| error.get("type"))
            .and_then(|c| c.as_str())
            .map(|s| s.to_string());

        Ok(StandardError {
            kind,
            message,
            code,
            retryable: matches!(kind, ErrorKind::RateLimit | ErrorKind::ServerError),
            fallback_hint: match kind {
                ErrorKind::RateLimit => Some(crate::protocol::error::FallbackHint::Retry),
                ErrorKind::ModelUnavailable => Some(crate::protocol::error::FallbackHint::FallbackTo("backup".into())),
                _ => None,
            },
        })
    }
}

impl OpenAiProtocol {
    fn pack_message(msg: &PrimitiveMessage) -> Value {
        let role = msg.role.to_string();
        let content_str = msg.content.iter().filter_map(|c| {
            if let crate::primitive::message::PrimitiveContent::Text { text } = c {
                Some(text.clone())
            } else {
                None
            }
        }).collect::<Vec<_>>().join("\n");
        json!({
            "role": role,
            "content": content_str
        })
    }
}
