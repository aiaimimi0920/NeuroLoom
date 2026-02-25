use serde_json::{json, Value};
use tokio_stream::StreamExt;
use reqwest::Response;

use crate::primitive::PrimitiveRequest;
use crate::provider::{LlmResponse, BoxLlmStream, LlmChunk};
use crate::protocol::traits::ProtocolFormat;
use crate::protocol::error::{StandardError, ErrorKind, FallbackHint};

pub struct DifyProtocol {}

impl ProtocolFormat for DifyProtocol {
    fn id(&self) -> &str {
        "dify"
    }

    fn pack(&self, req: &PrimitiveRequest, is_stream: bool) -> Value {
        // Dify 需要将历史消息组合为 query
        let mut query = String::new();
        
        if let Some(system) = &req.system {
            query.push_str("SYSTEM:\n");
            query.push_str(system);
            query.push_str("\n");
        }

        for msg in &req.messages {
            let role_prefix = match msg.role {
                crate::primitive::message::Role::User => "USER:\n",
                crate::primitive::message::Role::Assistant => "ASSISTANT:\n",
                crate::primitive::message::Role::System => "SYSTEM:\n",
                crate::primitive::message::Role::Tool => "TOOL:\n",
            };
            query.push_str(role_prefix);

            for content in &msg.content {
                match content {
                    crate::primitive::message::PrimitiveContent::Text { text } => {
                        query.push_str(text);
                    }
                    crate::primitive::message::PrimitiveContent::Image { .. } => {
                        query.push_str("[Image Omitted]");
                    }
                    crate::primitive::message::PrimitiveContent::ToolUse { name, .. } => {
                        query.push_str(&format!("[Invoking Tool: {}]", name));
                    }
                    crate::primitive::message::PrimitiveContent::ToolResult { content: tool_content, .. } => {
                        query.push_str(&format!("[Tool Result: {}]", tool_content));
                    }
                }
            }
            query.push_str("\n");
        }

        json!({
            "inputs": {},
            "query": query.trim(),
            "response_mode": if is_stream { "streaming" } else { "blocking" },
            "user": "nl_llm_v2_user"
        })
    }

    fn unpack_response(&self, raw: &str) -> anyhow::Result<LlmResponse> {
        let v: Value = serde_json::from_str(raw)?;

        if let Some(err_code) = v.get("code") {
            let message = v.get("message").and_then(|m| m.as_str()).unwrap_or("Unknown Dify error");
            return Err(anyhow::anyhow!("Dify API Error [{}]: {}", err_code, message));
        }

        let answer = v.get("answer").and_then(|a| a.as_str()).unwrap_or_default();
        
        Ok(LlmResponse {
            content: answer.to_string(),
            model: "dify".to_string(),
            usage: None,
        })
    }

    fn unpack_stream(&self, resp: Response) -> anyhow::Result<BoxLlmStream> {
        let stream = async_stream::stream! {
            let mut byte_stream = resp.bytes_stream();
            let mut buffer = String::new();

            while let Some(chunk_result) = byte_stream.next().await {
                let bytes = match chunk_result {
                    Ok(b) => b,
                    Err(e) => {
                        yield Err(anyhow::anyhow!("Http error during dify stream: {}", e));
                        continue;
                    }
                };

                let s = String::from_utf8_lossy(&bytes);
                buffer.push_str(&s);

                while let Some(pos) = buffer.find('\n') {
                    let line = buffer[..pos].trim().to_string();
                    buffer = buffer[pos + 1..].to_string();

                    let data_str = line.strip_prefix("data: ")
                        .or_else(|| line.strip_prefix("data:"));
                    
                    if let Some(data) = data_str {
                        let data = data.trim();
                        if data == "[DONE]" || data.is_empty() {
                            continue;
                        }

                        if let Ok(v) = serde_json::from_str::<Value>(data) {
                            if let Some(event) = v.get("event").and_then(|e| e.as_str()) {
                                if event == "message" || event == "agent_message" {
                                    if let Some(answer) = v.get("answer").and_then(|a| a.as_str()) {
                                        if !answer.is_empty() {
                                            yield Ok(LlmChunk {
                                                content: answer.to_string(),
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

    fn matches_format(&self, _data: &Value) -> bool {
        false // Not strictly OpenAI structure
    }

    fn unpack_error(&self, status: u16, raw: &str) -> anyhow::Result<StandardError> {
        let err_json: Result<Value, _> = serde_json::from_str(raw);
        let message = if let Ok(v) = err_json {
            v.get("message").and_then(|m| m.as_str()).unwrap_or(raw).to_string()
        } else {
            raw.to_string()
        };

        let kind = match status {
            400 => ErrorKind::Other,             // bad_request (invalid query)
            401 => ErrorKind::Authentication,    // unauthorized
            404 => ErrorKind::ModelUnavailable,  // not_found (app not found)
            429 => ErrorKind::RateLimit,         // rate_limit
            500..=599 => ErrorKind::ServerError, // internal_server_error
            _ => ErrorKind::Other,
        };

        Ok(StandardError {
            kind,
            message,
            code: Some(status.to_string()),
            retryable: matches!(kind, ErrorKind::RateLimit | ErrorKind::ServerError),
            fallback_hint: if status == 429 || status >= 500 {
                Some(FallbackHint::Retry)
            } else {
                None
            },
        })
    }
}
