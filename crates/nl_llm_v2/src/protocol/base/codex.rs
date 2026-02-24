use serde_json::{json, Value};
use tokio_stream::StreamExt;

use crate::protocol::traits::ProtocolFormat;
use crate::protocol::error::{StandardError, ErrorKind};
use crate::primitive::{PrimitiveRequest, PrimitiveMessage};
use crate::provider::{LlmResponse, BoxLlmStream, LlmChunk};

/// Codex 协议 (OpenAI Responses API 格式)
///
/// Codex 使用 /responses 端点，请求格式为：
/// ```json
/// {
///   "model": "gpt-5.1-codex",
///   "instructions": "...",       // system prompt
///   "input": "...",              // user messages
///   "stream": true               // Codex 默认 stream=true
/// }
/// ```
///
/// 响应格式为 SSE，最终事件 type=response.completed 中包含完整响应。
pub struct CodexProtocol;

impl ProtocolFormat for CodexProtocol {
    fn id(&self) -> &str {
        "codex"
    }

    fn pack(&self, primitive: &PrimitiveRequest, _is_stream: bool) -> Value {
        // 提取 system → instructions
        let instructions = primitive.system
            .as_deref()
            .unwrap_or("")
            .to_string();

        // 提取 user messages → input (拼接为单字符串 or 数组)
        let input = Self::build_input(&primitive.messages);

        let mut body = json!({
            "model": primitive.model,
            "input": input,
        });

        // instructions 只在非空时发送
        if !instructions.is_empty() {
            body["instructions"] = json!(instructions);
        } else {
            // Codex 要求 instructions 字段存在（即使为空）
            body["instructions"] = json!("");
        }

        // Codex Responses API (包括 ChatGPT backend) 强烈要求 stream=true
        // 即使是用 complete() 方法，我们也强制开启流式，然后在 unpack_response 中提取 SSE 的最后一个完成事件。
        body["stream"] = json!(true);
        // Codex API 特定强化配置
        body["store"] = json!(false);
        body["parallel_tool_calls"] = json!(true);
        body["include"] = json!(["reasoning.encrypted_content"]);

        // 注意：Codex Responses API 不支持 max_output_tokens, max_completion_tokens, temperature, top_p, service_tier
        // 所以这里不将 primitive.parameters 中的这些字段映射到 requests body 中。

        body
    }

    fn unpack_response(&self, raw: &str) -> anyhow::Result<LlmResponse> {
        // Codex 非流式响应的 response.completed 格式
        // 先尝试 SSE 解析（Codex 总是 stream=true，即使 complete 也可能返回 SSE）
        if raw.contains("data:") {
            if let Some(resp) = Self::parse_sse_completed(raw) {
                return Ok(resp);
            }
        }

        // 尝试直接 JSON 解析
        let v: Value = serde_json::from_str(raw)
            .map_err(|e| anyhow::anyhow!("Failed to parse Codex response JSON: {}", e))?;

        // Codex Responses API 格式
        let content = Self::extract_output_text(&v);
        let model = v.get("model")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown")
            .to_string();

        let usage = v.get("usage").map(|u| {
            crate::provider::Usage {
                prompt_tokens: u.get("input_tokens")
                    .or_else(|| u.get("prompt_tokens"))
                    .and_then(|t| t.as_u64()).unwrap_or(0) as u32,
                completion_tokens: u.get("output_tokens")
                    .or_else(|| u.get("completion_tokens"))
                    .and_then(|t| t.as_u64()).unwrap_or(0) as u32,
                total_tokens: u.get("total_tokens")
                    .and_then(|t| t.as_u64()).unwrap_or(0) as u32,
            }
        });

        Ok(LlmResponse { content, model, usage })
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

                    let data_str = line.strip_prefix("data: ")
                        .or_else(|| line.strip_prefix("data:"));
                    if let Some(data) = data_str {
                        let data = data.trim();
                        if data == "[DONE]" || data.is_empty() {
                            continue;
                        }

                        if let Ok(json) = serde_json::from_str::<Value>(data) {
                            let event_type = json.get("type")
                                .and_then(|t| t.as_str())
                                .unwrap_or("");

                            match event_type {
                                // 增量文本输出
                                "response.output_text.delta" => {
                                    if let Some(delta) = json.get("delta")
                                        .and_then(|d| d.as_str())
                                    {
                                        if !delta.is_empty() {
                                            yield Ok(LlmChunk {
                                                content: delta.to_string(),
                                            });
                                        }
                                    }
                                }
                                // 兼容 OpenAI chat completions SSE (fallback)
                                "" => {
                                    if let Some(choices) = json.get("choices").and_then(|c| c.as_array()) {
                                        if !choices.is_empty() {
                                            if let Some(delta_content) = choices[0]
                                                .get("delta")
                                                .and_then(|d| d.get("content"))
                                                .and_then(|c| c.as_str())
                                            {
                                                if !delta_content.is_empty() {
                                                    yield Ok(LlmChunk {
                                                        content: delta_content.to_string(),
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                                // response.completed → 可以忽略（流式不需要）
                                _ => {}
                            }
                        }
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }

    fn matches_format(&self, data: &Value) -> bool {
        data.get("input").is_some() && data.get("model").is_some()
    }

    fn unpack_error(&self, status: u16, raw: &str) -> anyhow::Result<StandardError> {
        let json: Value = serde_json::from_str(raw).unwrap_or_else(|_| json!({}));

        // Codex 错误可能在 "error" 或 "detail" 字段
        let error = &json["error"];
        let detail = json.get("detail").and_then(|d| d.as_str()).unwrap_or("");

        let kind = match status {
            401 | 403 => ErrorKind::Authentication,
            429 => ErrorKind::RateLimit,
            500..=599 => ErrorKind::ServerError,
            _ => ErrorKind::Other,
        };

        let message = error.get("message")
            .and_then(|m| m.as_str())
            .unwrap_or(if !detail.is_empty() { detail } else { raw })
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
                _ => None,
            },
        })
    }
}

impl CodexProtocol {
    /// 构建 input 字段：将 messages 转为 Codex 的 input 格式
    /// Codex API 要求 input 始终为数组格式，且内部结构为:
    /// {"type": "message", "role": "user", "content": [{"type": "input_text", "text": "..."}]}
    fn build_input(messages: &[PrimitiveMessage]) -> Value {
        let items: Vec<Value> = messages.iter().map(|msg| {
            // Codex 不支持 system role，需转为 developer
            let mut role = msg.role.to_string();
            if role == "system" {
                role = "developer".to_string();
            }
            
            let text = Self::extract_text(msg);
            json!({
                "type": "message",
                "role": role,
                "content": [
                    {
                        "type": "input_text",
                        "text": text
                    }
                ]
            })
        }).collect();
        json!(items)
    }

    fn extract_text(msg: &PrimitiveMessage) -> String {
        msg.content.iter().filter_map(|c| {
            if let crate::primitive::message::PrimitiveContent::Text { text } = c {
                Some(text.clone())
            } else {
                None
            }
        }).collect::<Vec<_>>().join("\n")
    }

    /// 从 Codex Responses API 响应中提取输出文本
    fn extract_output_text(v: &Value) -> String {
        // 尝试 output[].content[].text 格式
        if let Some(output) = v.get("output").and_then(|o| o.as_array()) {
            let texts: Vec<&str> = output.iter()
                .filter_map(|item| {
                    if item.get("type").and_then(|t| t.as_str()) == Some("message") {
                        item.get("content")
                            .and_then(|c| c.as_array())
                            .map(|content_arr| {
                                content_arr.iter()
                                    .filter_map(|c| c.get("text").and_then(|t| t.as_str()))
                                    .collect::<Vec<_>>()
                            })
                    } else {
                        None
                    }
                })
                .flatten()
                .collect();
            if !texts.is_empty() {
                return texts.join("");
            }
        }

        // Fallback: 直接 text 字段
        v.get("text")
            .and_then(|t| t.as_str())
            .unwrap_or_default()
            .to_string()
    }

    /// 从 SSE 文本中解析 response.completed 事件
    fn parse_sse_completed(raw: &str) -> Option<LlmResponse> {
        for line in raw.lines() {
            // SSE 格式包含 event:、data: 和空行，只处理 data: 行
            let data = match line.strip_prefix("data: ")
                .or_else(|| line.strip_prefix("data:"))
            {
                Some(d) => d.trim(),
                None => continue, // 跳过非 data: 行（event:, 空行等）
            };

            if data.is_empty() || data == "[DONE]" {
                continue;
            }

            if let Ok(json) = serde_json::from_str::<Value>(data) {
                if json.get("type").and_then(|t| t.as_str()) == Some("response.completed") {
                    if let Some(response) = json.get("response") {
                        let content = Self::extract_output_text(response);
                        let model = response.get("model")
                            .and_then(|m| m.as_str())
                            .unwrap_or("unknown")
                            .to_string();

                        let usage = response.get("usage").map(|u| {
                            crate::provider::Usage {
                                prompt_tokens: u.get("input_tokens")
                                    .or_else(|| u.get("prompt_tokens"))
                                    .and_then(|t| t.as_u64()).unwrap_or(0) as u32,
                                completion_tokens: u.get("output_tokens")
                                    .or_else(|| u.get("completion_tokens"))
                                    .and_then(|t| t.as_u64()).unwrap_or(0) as u32,
                                total_tokens: u.get("total_tokens")
                                    .and_then(|t| t.as_u64()).unwrap_or(0) as u32,
                            }
                        });

                        return Some(LlmResponse {
                            content,
                            model,
                            usage,
                        });
                    }
                }
            }
        }
        None
    }
}
