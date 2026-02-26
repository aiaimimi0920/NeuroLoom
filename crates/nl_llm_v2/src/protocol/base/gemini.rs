use serde_json::{json, Value};
use tokio_stream::StreamExt;

use crate::primitive::{PrimitiveMessage, PrimitiveRequest};
use crate::protocol::error::{ErrorKind, StandardError};
use crate::protocol::traits::ProtocolFormat;
use crate::provider::{BoxLlmStream, LlmChunk, LlmResponse};

/// Gemini 标准协议封包与解包
pub struct GeminiProtocol;

impl ProtocolFormat for GeminiProtocol {
    fn id(&self) -> &str {
        "gemini"
    }

    fn pack(&self, primitive: &PrimitiveRequest, _is_stream: bool) -> Value {
        // Gemini 的基础结构为:
        // { "contents": [ ... ], "systemInstruction": { "parts": [ {"text": "..."} ] } }
        let mut body = json!({
            "contents": primitive.messages.iter().map(Self::pack_message).collect::<Vec<_>>(),
        });

        if let Some(sys) = &primitive.system {
            body["systemInstruction"] = json!({
                "parts": [{ "text": sys }]
            });
        }

        // Apply generation config parameters
        let params = &primitive.parameters;
        let mut gen_config = json!({});

        if let Some(temp) = params.temperature {
            gen_config["temperature"] = json!(temp);
        }
        if let Some(top_p) = params.top_p {
            gen_config["topP"] = json!(top_p);
        }
        if let Some(max_tok) = params.max_tokens {
            gen_config["maxOutputTokens"] = json!(max_tok);
        }

        if !gen_config.as_object().unwrap().is_empty() {
            body["generationConfig"] = gen_config;
        }

        body
    }

    fn unpack_response(&self, raw: &str) -> anyhow::Result<LlmResponse> {
        let v: Value = serde_json::from_str(raw)
            .map_err(|e| anyhow::anyhow!("Failed to parse Gemini JSON: {}\nRaw: {}", e, raw))?;

        // [修复] 处理可能包含一层 "response" 包装的格式 (例如 CloudCode)
        let candidates = v
            .get("response")
            .and_then(|r| r.get("candidates"))
            .or_else(|| v.get("candidates"));

        let content = candidates
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("content"))
            .and_then(|c| c.get("parts"))
            .and_then(|p| p.get(0))
            .and_then(|p| p.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or_default()
            .to_string();

        // [修复] 正确解析 model（Gemini 响应中可能没有，使用默认值）
        let model = v
            .get("model")
            .and_then(|m| m.as_str())
            .unwrap_or("gemini")
            .to_string();

        // [修复] 正确解析 usageMetadata
        let usage = v.get("usageMetadata").map(|u| crate::provider::Usage {
            prompt_tokens: u
                .get("promptTokenCount")
                .and_then(|t| t.as_u64())
                .unwrap_or(0) as u32,
            completion_tokens: u
                .get("candidatesTokenCount")
                .and_then(|t| t.as_u64())
                .unwrap_or(0) as u32,
            total_tokens: u
                .get("totalTokenCount")
                .and_then(|t| t.as_u64())
                .unwrap_or(0) as u32,
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

                // Check if this might be a pure non-SSE JSON response returned by mistake
                if buffer.starts_with('{') && buffer.ends_with('}') {
                    if let Ok(json) = serde_json::from_str::<Value>(&buffer) {
                        let candidates = json.get("response")
                            .and_then(|r| r.get("candidates"))
                            .or_else(|| json.get("candidates"));

                        if let Some(text) = candidates
                            .and_then(|c| c.get(0))
                            .and_then(|c| c.get("content"))
                            .and_then(|c| c.get("parts"))
                            .and_then(|p| p.get(0))
                            .and_then(|p| p.get("text"))
                            .and_then(|t| t.as_str())
                        {
                            if !text.is_empty() {
                                yield Ok(LlmChunk {
                                    content: text.to_string(),
                                });
                            }
                            buffer.clear();
                            return;
                        }
                    }
                }

                while let Some(pos) = buffer.find('\n') {
                    let line = buffer[..pos].trim().to_string();
                    buffer = buffer[pos + 1..].to_string();

                    if let Some(data) = line.strip_prefix("data: ") {
                        let data = data.trim();
                        if data == "[DONE]" || data.is_empty() {
                            continue;
                        }

                        if let Ok(json) = serde_json::from_str::<Value>(data) {
                            let candidates = json.get("response")
                                .and_then(|r| r.get("candidates"))
                                .or_else(|| json.get("candidates"));

                            if let Some(text) = candidates
                                .and_then(|c| c.get(0))
                                .and_then(|c| c.get("content"))
                                .and_then(|c| c.get("parts"))
                                .and_then(|p| p.get(0))
                                .and_then(|p| p.get("text"))
                                .and_then(|t| t.as_str())
                            {
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
        };

        Ok(Box::pin(stream))
    }

    fn matches_format(&self, data: &Value) -> bool {
        data.get("contents").is_some()
    }

    fn unpack_error(&self, status: u16, raw: &str) -> anyhow::Result<StandardError> {
        // [修复] 解析 Gemini 错误 JSON 获取详细信息
        let json: Value = serde_json::from_str(raw).unwrap_or_else(|_| json!({}));
        let error = &json["error"];

        let kind = match status {
            401 | 403 => ErrorKind::Authentication,
            429 => ErrorKind::RateLimit,
            500..=599 => ErrorKind::ServerError,
            _ => match error.get("status").and_then(|s| s.as_str()) {
                Some("RESOURCE_EXHAUSTED") => ErrorKind::RateLimit,
                Some("INVALID_ARGUMENT") => ErrorKind::ContextLengthExceeded,
                Some("NOT_FOUND") => ErrorKind::ModelUnavailable,
                _ => ErrorKind::Other,
            },
        };

        let message = error
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or(raw)
            .to_string();

        // [修复] 正确处理 code 字段
        let code = error
            .get("status")
            .and_then(|s| s.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                error
                    .get("code")
                    .and_then(|c| c.as_u64())
                    .map(|n| n.to_string())
            });

        Ok(StandardError {
            kind,
            message,
            code,
            retryable: matches!(kind, ErrorKind::RateLimit | ErrorKind::ServerError),
            fallback_hint: match kind {
                ErrorKind::RateLimit => Some(crate::protocol::error::FallbackHint::Retry),
                _ => None,
            },
        })
    }
}

impl GeminiProtocol {
    fn pack_message(msg: &PrimitiveMessage) -> Value {
        // Gemini uses "user" and "model" strictly usually. Provide standard mapping.
        let role_str = msg.role.to_string();
        let role = match role_str.as_str() {
            "assistant" => "model",
            other => other,
        };
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
            "parts": [{ "text": content_str }]
        })
    }
}
