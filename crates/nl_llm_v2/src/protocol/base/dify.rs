use serde_json::{json, Value};
use tokio_stream::StreamExt;
use reqwest::Response;

use crate::primitive::PrimitiveRequest;
use crate::provider::{LlmResponse, BoxLlmStream, LlmChunk};
use crate::protocol::traits::ProtocolFormat;
use crate::protocol::error::{StandardError, ErrorKind, FallbackHint};

pub struct DifyProtocol {}

impl DifyProtocol {
    fn infer_user(req: &PrimitiveRequest) -> String {
        req.metadata
            .user_id
            .as_deref()
            .or_else(|| req.metadata.session_id.as_deref())
            .unwrap_or("nl_llm_v2")
            .to_string()
    }

    fn body_error(v: &Value) -> Option<StandardError> {
        // Dify 可能出现 HTTP 200 但 JSON body 内携带业务错误：{ code, message }
        // 为避免误判，仅在 code + message 同时存在时认为是错误。
        let code = v.get("code").and_then(Self::value_to_code)?;
        let message = v.get("message").and_then(Self::value_to_message)?;
        Some(Self::standard_error_from_code_message(Some(code), message))
    }

    fn value_to_code(v: &Value) -> Option<String> {
        match v {
            Value::String(s) => Some(s.clone()),
            Value::Number(n) => Some(n.to_string()),
            _ => None,
        }
    }

    fn value_to_message(v: &Value) -> Option<String> {
        match v {
            Value::String(s) => Some(s.clone()),
            _ => Some(v.to_string()),
        }
    }

    fn standard_error_from_code_message(code: Option<String>, message: String) -> StandardError {
        let msg = message.to_lowercase();
        let code_str = code.as_deref().unwrap_or("");
        let code_lc = code_str.to_lowercase();

        let is_5xx = match code_str.parse::<u16>() {
            Ok(s) => s >= 500,
            Err(_) => false,
        };

        let kind = if code_str == "401"
            || code_str == "403"
            || msg.contains("unauthorized")
            || msg.contains("forbidden")
            || msg.contains("api key")
            || msg.contains("invalid_api_key")
            || msg.contains("invalid api key")
            || msg.contains("authentication")
            || code_lc.contains("unauthorized")
            || code_lc.contains("invalid_api_key")
        {
            ErrorKind::Authentication
        } else if code_str == "429"
            || msg.contains("429")
            || msg.contains("rate limit")
            || msg.contains("too many requests")
            || code_lc.contains("rate_limit")
            || code_lc.contains("quota")
        {
            ErrorKind::RateLimit
        } else if msg.contains("timeout") || msg.contains("timed out") || code_lc.contains("timeout") {
            ErrorKind::Timeout
        } else if msg.contains("connection") || msg.contains("network") || msg.contains("dns") {
            ErrorKind::Network
        } else if is_5xx || msg.contains("server error") || msg.contains("internal") || msg.contains("502") || msg.contains("503") {
            ErrorKind::ServerError
        } else if msg.contains("not found") || code_lc.contains("not_found") {
            ErrorKind::ModelUnavailable
        } else if (msg.contains("context") && msg.contains("length"))
            || msg.contains("maximum context")
            || code_lc.contains("context_length")
        {
            ErrorKind::ContextLengthExceeded
        } else if msg.contains("content filter") || msg.contains("safety") || code_lc.contains("content_filter") {
            ErrorKind::ContentFilter
        } else {
            ErrorKind::Other
        };

        let retryable = matches!(
            kind,
            ErrorKind::RateLimit | ErrorKind::Timeout | ErrorKind::ServerError | ErrorKind::Network
        );

        let fallback_hint = match kind {
            ErrorKind::RateLimit | ErrorKind::Timeout | ErrorKind::ServerError | ErrorKind::Network => {
                Some(FallbackHint::Retry)
            }
            ErrorKind::ContextLengthExceeded | ErrorKind::ModelUnavailable => {
                Some(FallbackHint::DowngradeModel)
            }
            _ => None,
        };

        StandardError {
            kind,
            message,
            code,
            retryable,
            fallback_hint,
        }
    }
}

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
            "user": Self::infer_user(req)
        })
    }

    fn unpack_response(&self, raw: &str) -> anyhow::Result<LlmResponse> {
        let v: Value = serde_json::from_str(raw)?;

        if let Some(se) = Self::body_error(&v) {
            return Err(anyhow::anyhow!("{}", se));
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
                            if let Some(se) = DifyProtocol::body_error(&v) {
                                yield Err(anyhow::anyhow!("{}", se));
                                break;
                            }

                            if let Some(event) = v.get("event").and_then(|e| e.as_str()) {
                                if event == "error" {
                                    // 部分 Dify SSE 可能通过 event=error 输出错误 payload
                                    let code = v.get("code").and_then(DifyProtocol::value_to_code);
                                    let msg = v
                                        .get("message")
                                        .and_then(DifyProtocol::value_to_message)
                                        .unwrap_or_else(|| "Unknown Dify error".to_string());
                                    let se = DifyProtocol::standard_error_from_code_message(code, msg);
                                    yield Err(anyhow::anyhow!("{}", se));
                                    break;
                                }

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
            retryable: matches!(
                kind,
                ErrorKind::RateLimit | ErrorKind::ServerError | ErrorKind::Network | ErrorKind::Timeout
            ),
            fallback_hint: match kind {
                ErrorKind::RateLimit | ErrorKind::ServerError | ErrorKind::Network | ErrorKind::Timeout => {
                    Some(FallbackHint::Retry)
                }
                ErrorKind::ContextLengthExceeded | ErrorKind::ModelUnavailable => {
                    Some(FallbackHint::DowngradeModel)
                }
                _ => None,
            },
        })
    }
}
