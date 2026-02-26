use serde_json::{json, Value};
use tokio_stream::StreamExt;
use reqwest::Response;

use crate::primitive::PrimitiveRequest;
use crate::provider::{LlmResponse, BoxLlmStream, LlmChunk};
use crate::protocol::traits::ProtocolFormat;
use crate::protocol::error::{StandardError, ErrorKind, FallbackHint};

pub struct CozeProtocol {}

impl CozeProtocol {
    fn infer_user(req: &PrimitiveRequest) -> String {
        req.metadata
            .user_id
            .as_deref()
            .or_else(|| req.metadata.session_id.as_deref())
            .unwrap_or("nl_llm_v2_user")
            .to_string()
    }
}

impl ProtocolFormat for CozeProtocol {
    fn id(&self) -> &str {
        "coze"
    }

    fn pack(&self, req: &PrimitiveRequest, _is_stream: bool) -> Value {
        // Coze API requires bot_id instead of model in the standard OpenAI sense, so we expect the user to pass bot_id as the model name.
        let bot_id = req.model.clone();

        // Convert messages to Coze format
        let mut additional_messages = Vec::new();
        for msg in &req.messages {
            let role = match msg.role {
                crate::primitive::message::Role::User => "user",
                crate::primitive::message::Role::Assistant => "assistant",
                crate::primitive::message::Role::System | crate::primitive::message::Role::Tool => "user", // Coze V3 additional_messages mostly supports user, converting everything to user/assistant semantics for simplicity
            };

            let mut content_str = String::new();
            for part in &msg.content {
                match part {
                    crate::primitive::message::PrimitiveContent::Text { text } => content_str.push_str(text),
                    _ => content_str.push_str("[Unsupported Content] "),
                }
            }

            additional_messages.push(json!({
                "role": role,
                "content": content_str,
                "content_type": "text"
            }));
        }

        // Add system message if present
        if let Some(sys) = &req.system {
            additional_messages.insert(0, json!({
                "role": "user",
                "content": format!("System Directive: {}", sys),
                "content_type": "text"
            }));
        }

        json!({
            "bot_id": bot_id,
            "user_id": Self::infer_user(req),
            "additional_messages": additional_messages,
            "stream": true // Always enforce true to bypass complex polling for non-stream blocks
        })
    }

    fn unpack_response(&self, raw: &str) -> anyhow::Result<LlmResponse> {
        // Since we injected `stream: true` unconditionally, `raw` here is technically an aggregated string from our stream interception, 
        // OR an error JSON payload if it failed immediately before streaming.
        
        let mut final_content = String::new();
        let model_name = "coze".to_string();

        if raw.starts_with('{') {
            // It might be a direct error JSON payload if the initial POST failed (e.g. 400 Bad Request, 401 Unauthorized)
            if let Ok(value) = serde_json::from_str::<Value>(raw) {
                if value.get("code").is_some() || value.get("error").is_some() {
                    let err_msg = value.get("msg").and_then(|m| m.as_str())
                        .or_else(|| value.get("error").and_then(|e| e.as_str()))
                        .unwrap_or(raw);
                    return Err(anyhow::anyhow!("Coze API Error: {}", err_msg));
                }
            }
        }

        // Since the pipeline might deliver the raw SSE frames due to how UnpackStage works currently, 
        // we might need to parse SSE here if SendStage didn't abstract it. 
        // If SendStage buffered the SSE and returns it directly to `unpack_response`, it's just raw SSE texts.
        for line in raw.lines() {
            let line = line.trim();
            if line.starts_with("data:") {
                let data = line.trim_start_matches("data:").trim();
                if data == "[DONE]" || data.is_empty() {
                    continue;
                }
                if let Ok(v) = serde_json::from_str::<Value>(data) {
                    if let Some(event) = v.get("event").and_then(|e| e.as_str()) {
                        if event == "conversation.message.delta" {
                            if let Some(msg_data) = v.get("message") {
                                if msg_data.get("type").and_then(|t| t.as_str()) == Some("answer") {
                                    if let Some(content) = msg_data.get("content").and_then(|c| c.as_str()) {
                                        final_content.push_str(content);
                                    }
                                }
                            }
                        } else if event == "error" {
                             let code = v.get("error_info").and_then(|e| e.get("error_code")).and_then(|c| c.as_i64()).unwrap_or(0);
                             let msg = v.get("error_info").and_then(|e| e.get("error_message")).and_then(|m| m.as_str()).unwrap_or("Unknown error");
                             return Err(anyhow::anyhow!("Coze stream error [{}]: {}", code, msg));
                        }
                    }
                    // For "conversation.chat.completed", it contains usage which we could theoretically parse
                }
            }
        }
        
        Ok(LlmResponse {
            content: final_content,
            model: model_name,
            usage: None,
        })
    }

    fn unpack_stream(&self, resp: Response) -> anyhow::Result<BoxLlmStream> {
        let stream = async_stream::stream! {
            let mut byte_stream = resp.bytes_stream();
            let mut buffer = String::new();
            let mut current_event = String::new();
            let mut first_chunk = true;

            while let Some(chunk_result) = byte_stream.next().await {
                let bytes = match chunk_result {
                    Ok(b) => b,
                    Err(e) => {
                        yield Err(anyhow::anyhow!("Http error during Coze stream: {}", e));
                        continue;
                    }
                };

                let s = String::from_utf8_lossy(&bytes);
                buffer.push_str(&s);

                // [关键修复] Coze 在 HTTP 200 下也会返回 JSON 错误体（非 SSE）
                // 例如 bot_id 不存在时返回 {"code":4200,"msg":"..."}
                // 必须在第一个 chunk 中检测这种情况
                if first_chunk {
                    first_chunk = false;
                    let trimmed = buffer.trim_start();
                    if trimmed.starts_with('{') {
                        // 可能是 JSON 错误而非 SSE 流
                        if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
                            if let Some(code) = v.get("code").and_then(|c| c.as_i64()) {
                                let msg = v.get("msg").and_then(|m| m.as_str()).unwrap_or("Unknown Coze error");
                                yield Err(anyhow::anyhow!("Coze API Error ({}): {}", code, msg));
                                return; // 立即终止流
                            }
                        }
                    }
                }

                while let Some(pos) = buffer.find('\n') {
                    let line = buffer[..pos].trim().to_string();
                    buffer = buffer[pos + 1..].to_string();

                    if line.is_empty() {
                        continue;
                    }

                    if line.starts_with("event:") {
                        current_event = line.trim_start_matches("event:").trim().to_string();
                        continue;
                    }

                    if line.starts_with("data:") {
                        let data = line.trim_start_matches("data:").trim();
                        if data == "[DONE]" || data.is_empty() {
                            continue;
                        }

                        if current_event == "conversation.message.delta" {
                            if let Ok(v) = serde_json::from_str::<Value>(data) {
                                if v.get("type").and_then(|t| t.as_str()) == Some("answer") {
                                    if let Some(content) = v.get("content").and_then(|c| c.as_str()) {
                                        if !content.is_empty() {
                                            yield Ok(LlmChunk {
                                                content: content.to_string(),
                                            });
                                        }
                                    }
                                }
                            }
                        } else if current_event == "error" {
                            if let Ok(v) = serde_json::from_str::<Value>(data) {
                                let code = v.get("code").and_then(|c| c.as_i64()).unwrap_or(0);
                                let msg = v.get("msg").and_then(|m| m.as_str()).unwrap_or("Unknown stream error");
                                yield Err(anyhow::anyhow!("Coze Stream Error ({}): {}", code, msg));
                            }
                        } else if current_event == "done" {
                            // 流结束信号，可以安全退出
                            return;
                        }

                        // "conversation.chat.completed" could be parsed for Usage tracking in the future
                        current_event.clear();
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }

    fn matches_format(&self, _data: &Value) -> bool {
        false // Coze does not return OpenAI format chunks natively
    }

    fn unpack_error(&self, status: u16, raw: &str) -> anyhow::Result<StandardError> {
        let err_json: Result<Value, _> = serde_json::from_str(raw);
        let message;
        let mut extracted_code = Some(status.to_string());

        if let Ok(v) = err_json {
            if let Some(msg) = v.get("msg").and_then(|m| m.as_str()) {
                message = msg.to_string();
            } else if let Some(msg) = v.get("error_message").and_then(|m| m.as_str()) {
                message = msg.to_string();
            } else {
                message = raw.to_string();
            }

            if let Some(code) = v.get("code").and_then(|c| c.as_i64()) {
                extracted_code = Some(code.to_string());
            }
        } else {
            message = raw.to_string();
        }

        let kind = match status {
            400 => ErrorKind::Other,
            401 => ErrorKind::Authentication,
            403 => ErrorKind::Authentication,
            404 => ErrorKind::ModelUnavailable,
            429 => ErrorKind::RateLimit,
            500..=599 => ErrorKind::ServerError,
            _ => {
                // Determine via code payload
                if let Some(ref c) = extracted_code {
                    if c == "4001" || c == "4002" {
                        ErrorKind::Authentication
                    } else if c == "4100" {
                        ErrorKind::RateLimit
                    } else if c == "4200" {
                        ErrorKind::ModelUnavailable // Bot not found
                    } else {
                        ErrorKind::Other
                    }
                } else {
                    ErrorKind::Other
                }
            }
        };

        Ok(StandardError {
            kind,
            message,
            code: extracted_code,
            retryable: matches!(
                kind,
                ErrorKind::RateLimit | ErrorKind::ServerError | ErrorKind::Network | ErrorKind::Timeout
            ),
            fallback_hint: match kind {
                 ErrorKind::RateLimit | ErrorKind::ServerError | ErrorKind::Network | ErrorKind::Timeout => Some(FallbackHint::Retry),
                 ErrorKind::ModelUnavailable | ErrorKind::ContextLengthExceeded => Some(FallbackHint::DowngradeModel),
                 _ => None,
            },
        })
    }
}
