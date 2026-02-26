use crate::primitive::{PrimitiveMessage, PrimitiveRequest};
use crate::protocol::error::{ErrorKind, StandardError};
use crate::protocol::traits::ProtocolFormat;
use crate::provider::{BoxLlmStream, LlmChunk, LlmResponse};
use anyhow::Result;
use serde_json::{json, Value};
use tokio_stream::StreamExt;

pub struct ClaudeProtocol;

impl ProtocolFormat for ClaudeProtocol {
    fn id(&self) -> &str {
        "claude"
    }

    fn pack(&self, primitive: &PrimitiveRequest, is_stream: bool) -> Value {
        // Anthropic Claude API format
        let mut body = json!({
            "model": primitive.model,
            "messages": primitive.messages.iter().map(Self::pack_message).collect::<Vec<_>>(),
        });

        if is_stream {
            body["stream"] = json!(true);
        }

        // [修复] Claude 的 system 应该是字符串格式（根据官方 API 文档）
        // 注：设计规范中写的是数组格式，但实际 Claude API 使用字符串
        if let Some(sys) = &primitive.system {
            body["system"] = json!(sys);
        }

        // Apply parameters (Claude specific generation configurations)
        let params = &primitive.parameters;
        if let Some(temp) = params.temperature {
            body["temperature"] = json!(temp);
        }
        if let Some(top_p) = params.top_p {
            body["top_p"] = json!(top_p);
        }

        // Anthropic requires max_tokens to be set (defaults to 1024 if not specified in our primitives typically but let's be explicit if present)
        body["max_tokens"] = json!(params.max_tokens.unwrap_or(4096));

        body
    }

    fn unpack_response(&self, raw: &str) -> Result<LlmResponse> {
        let v: Value = serde_json::from_str(raw)
            .map_err(|e| anyhow::anyhow!("Failed to parse Claude JSON: {}", e))?;

        // Claude typically returns: { "content": [ { "text": "...", "type": "text" } ] }
        let content_arr = v.get("content").and_then(|c| c.as_array());

        let mut final_text = String::new();
        if let Some(arr) = content_arr {
            for item in arr {
                if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                    if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                        final_text.push_str(text);
                    }
                }
            }
        }

        // [修复] 正确解析 model
        let model = v
            .get("model")
            .and_then(|m| m.as_str())
            .unwrap_or("claude")
            .to_string();

        // [修复] 正确解析 usage
        let usage = v.get("usage").map(|u| {
            crate::provider::Usage {
                prompt_tokens: u.get("input_tokens").and_then(|t| t.as_u64()).unwrap_or(0) as u32,
                completion_tokens: u.get("output_tokens").and_then(|t| t.as_u64()).unwrap_or(0)
                    as u32,
                total_tokens: 0, // Claude 不直接返回 total
            }
        });

        Ok(LlmResponse {
            content: final_text,
            model,
            usage,
        })
    }

    fn unpack_stream(&self, resp: reqwest::Response) -> Result<BoxLlmStream> {
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

                    if let Some(data) = line.strip_prefix("data: ") {
                        let data = data.trim();
                        if data.is_empty() {
                            continue;
                        }

                        if let Ok(json) = serde_json::from_str::<Value>(data) {
                            // Claude stream format often has: {"type": "content_block_delta", "delta": {"type": "text_delta", "text": "..."}}
                            let event_type = json.get("type").and_then(|t| t.as_str());

                            if event_type == Some("content_block_delta") {
                                if let Some(delta) = json.get("delta") {
                                    if delta.get("type").and_then(|t| t.as_str()) == Some("text_delta") {
                                        if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
                                            if !text.is_empty() {
                                                yield Ok(LlmChunk {
                                                    content: text.to_string(),
                                                });
                                            }
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
        // [修复] Claude 特征检查：存在 model 且格式符合 Claude API
        // 原因：system 可能是字符串或数组，不应作为唯一判断条件
        data.get("model").is_some()
            && data.get("messages").is_some()
            // Claude 特有：max_tokens 必须存在（与其他 API 不同）
            && data.get("max_tokens").is_some()
    }

    fn unpack_error(&self, status: u16, raw: &str) -> Result<StandardError> {
        let json: Value = serde_json::from_str(raw).unwrap_or_else(|_| json!({}));
        let error = &json["error"];

        let kind = match status {
            401 | 403 => ErrorKind::Authentication,
            429 => ErrorKind::RateLimit,
            _ => match error["type"].as_str() {
                Some("context_length_exceeded") => ErrorKind::ContextLengthExceeded,
                Some("content_filter") => ErrorKind::ContentFilter,
                _ => ErrorKind::Other,
            },
        };

        Ok(StandardError {
            kind,
            message: error["message"]
                .as_str()
                .unwrap_or("Unknown Claude Error")
                .to_string(),
            code: error["type"].as_str().map(|s| s.to_string()),
            retryable: matches!(kind, ErrorKind::RateLimit | ErrorKind::ServerError),
            fallback_hint: None,
        })
    }
}

impl ClaudeProtocol {
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
            "role": role,
            "content": content_str
        })
    }
}
