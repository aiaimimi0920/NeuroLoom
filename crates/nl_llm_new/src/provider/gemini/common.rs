//! Gemini 协议共享代码
//!
//! 提供以下共享函数：
//! - `compile_request`: 将 PrimitiveRequest 编译为 Gemini JSON 请求体
//! - `parse_response`: 解析 Gemini 非流式响应
//! - `parse_sse_stream`: 解析 Gemini SSE 流式响应

use crate::primitive::{PrimitiveContent, PrimitiveRequest, Role};
use crate::provider::{LlmChunk, LlmResponse, StopReason, Usage};
use serde_json::{json, Value};

/// 将 PrimitiveRequest 编译为 Gemini/Vertex JSON 请求体
///
/// Gemini native 格式：
/// - role: "user" | "model"（不用 "assistant"）
/// - parts: [{ "text": "..." }]
/// - systemInstruction: { "parts": [...] }
pub fn compile_request(primitive: &PrimitiveRequest) -> Value {
    let mut body = json!({});

    // System instruction
    if let Some(system) = &primitive.system {
        body["systemInstruction"] = json!({
            "parts": [{"text": system}]
        });
    }

    // Contents (messages)
    let contents: Vec<Value> = primitive
        .messages
        .iter()
        .map(|msg| {
            let role = match msg.role {
                Role::User => "user",
                Role::Assistant => "model", // Gemini uses "model" not "assistant"
                Role::System => "user",
            };

            let parts: Vec<Value> = msg
                .content
                .iter()
                .map(|c| match c {
                    PrimitiveContent::Text { text } => json!({"text": text}),
                    PrimitiveContent::Image { mime_type, data } => json!({
                        "inlineData": { "mimeType": mime_type, "data": data }
                    }),
                    PrimitiveContent::ToolCall { name, arguments, .. } => json!({
                        "functionCall": { "name": name, "args": arguments }
                    }),
                    PrimitiveContent::ToolResult {
                        tool_call_id,
                        content,
                        ..
                    } => json!({
                        "functionResponse": { "name": tool_call_id, "response": {"result": content} }
                    }),
                    PrimitiveContent::Thinking { text } => json!({"text": text}),
                })
                .collect();

            json!({"role": role, "parts": parts})
        })
        .collect();
    body["contents"] = json!(contents);

    // Generation config
    let mut gen_config = json!({});
    if let Some(max_tokens) = primitive.parameters.max_tokens {
        gen_config["maxOutputTokens"] = json!(max_tokens);
    }
    if let Some(temperature) = primitive.parameters.temperature {
        gen_config["temperature"] = json!(temperature);
    }
    if let Some(top_p) = primitive.parameters.top_p {
        gen_config["topP"] = json!(top_p);
    }
    if let Some(stop) = &primitive.parameters.stop_sequences {
        gen_config["stopSequences"] = json!(stop);
    }
    if gen_config != json!({}) {
        body["generationConfig"] = gen_config;
    }

    // Tools
    if !primitive.tools.is_empty() {
        let func_decls: Vec<Value> = primitive
            .tools
            .iter()
            .map(|t| {
                json!({
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.input_schema,
                })
            })
            .collect();
        body["tools"] = json!([{"functionDeclarations": func_decls}]);
    }

    body
}

/// 解析 Gemini 非流式响应：candidates[0].content.parts[0].text
pub fn parse_response(raw: &str) -> crate::Result<LlmResponse> {
    let json: Value = serde_json::from_str(raw).map_err(|e| {
        crate::Error::Provider(crate::provider::ProviderError::fail(format!(
            "gemini: generateContent decode response failed: {}",
            e
        )))
    })?;

    let content = json
        .get("candidates")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("content"))
        .and_then(|c| c.get("parts"))
        .and_then(|p| p.get(0))
        .and_then(|p| p.get("text"))
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();

    // 解析 usage
    let usage = json
        .get("usageMetadata")
        .map(|u| Usage {
            input_tokens: u.get("promptTokenCount").and_then(|v| v.as_u64()).unwrap_or(0),
            output_tokens: u.get("candidatesTokenCount").and_then(|v| v.as_u64()).unwrap_or(0),
            thinking_tokens: u.get("thoughtsTokenCount").and_then(|v| v.as_u64()),
        })
        .unwrap_or_default();

    // 解析 stop_reason
    let finish_reason = json
        .get("candidates")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("finishReason"))
        .and_then(|r| r.as_str())
        .unwrap_or("STOP");

    let stop_reason = match finish_reason {
        "STOP" => StopReason::EndTurn,
        "MAX_TOKENS" => StopReason::MaxTokens,
        "SAFETY" => StopReason::StopSequence,
        "RECITATION" => StopReason::StopSequence,
        _ => StopReason::EndTurn,
    };

    Ok(LlmResponse {
        content,
        tool_calls: Vec::new(),
        usage,
        stop_reason,
    })
}

/// 解析 SSE 流，返回 BoxStream
pub fn parse_sse_stream(
    resp: reqwest::Response,
) -> std::pin::Pin<Box<dyn futures::Stream<Item = crate::Result<LlmChunk>> + Send + 'static>> {
    use futures::StreamExt;

    let stream = async_stream::stream! {
        let mut byte_stream = resp.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk) = byte_stream.next().await {
            let bytes = match chunk {
                Ok(b) => b,
                Err(e) => {
                    yield Err(crate::Error::Http(e.to_string()));
                    continue;
                }
            };
            buffer.push_str(&String::from_utf8_lossy(&bytes));

            // 按行处理
            while let Some(pos) = buffer.find('\n') {
                let line = buffer[..pos].trim().to_string();
                buffer = buffer[pos + 1..].to_string();

                if let Some(data) = line.strip_prefix("data: ") {
                    let data = data.trim();
                    if data == "[DONE]" || data.is_empty() {
                        continue;
                    }
                    if let Ok(json) = serde_json::from_str::<Value>(data) {
                        if let Some(text) = json
                            .get("candidates")
                            .and_then(|c| c.get(0))
                            .and_then(|c| c.get("content"))
                            .and_then(|c| c.get("parts"))
                            .and_then(|p| p.get(0))
                            .and_then(|p| p.get("text"))
                            .and_then(|t| t.as_str())
                        {
                            if !text.is_empty() {
                                yield Ok(LlmChunk {
                                    delta: crate::provider::ChunkDelta::Text(text.to_string()),
                                    usage: None,
                                });
                            }
                        }
                    }
                }
            }
        }
    };

    Box::pin(stream)
}

// ── 测试 ─────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitive::PrimitiveMessage;

    #[test]
    fn test_compile_request_user_message() {
        let primitive = PrimitiveRequest {
            model: "gemini-2.5-flash".to_string(),
            messages: vec![PrimitiveMessage::user("Hello Gemini!")],
            ..Default::default()
        };
        let body = compile_request(&primitive);

        let contents = body["contents"].as_array().unwrap();
        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0]["role"], "user");
        assert_eq!(contents[0]["parts"][0]["text"], "Hello Gemini!");
    }

    #[test]
    fn test_compile_request_system_message() {
        let primitive = PrimitiveRequest {
            model: "gemini-2.5-flash".to_string(),
            system: Some("You are a helpful AI.".to_string()),
            messages: vec![PrimitiveMessage::user("Hello!")],
            ..Default::default()
        };
        let body = compile_request(&primitive);

        // systemInstruction 应该被提取
        assert!(body.get("systemInstruction").is_some());
        let parts = body["systemInstruction"]["parts"].as_array().unwrap();
        assert_eq!(parts[0]["text"], "You are a helpful AI.");

        // contents 只有 user
        let contents = body["contents"].as_array().unwrap();
        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0]["role"], "user");
    }

    #[test]
    fn test_compile_request_assistant_message() {
        let primitive = PrimitiveRequest {
            model: "gemini-2.5-flash".to_string(),
            messages: vec![
                PrimitiveMessage::user("Hi"),
                PrimitiveMessage::assistant("Hello!"),
                PrimitiveMessage::user("How are you?"),
            ],
            ..Default::default()
        };
        let body = compile_request(&primitive);

        let contents = body["contents"].as_array().unwrap();
        assert_eq!(contents.len(), 3);
        assert_eq!(contents[0]["role"], "user");
        assert_eq!(contents[1]["role"], "model"); // assistant -> model
        assert_eq!(contents[2]["role"], "user");
    }

    #[test]
    fn test_parse_response() {
        let raw = r#"{
            "candidates": [{
                "content": {
                    "parts": [{ "text": "Hello, I am Gemini!" }],
                    "role": "model"
                },
                "finishReason": "STOP"
            }],
            "usageMetadata": {
                "promptTokenCount": 10,
                "candidatesTokenCount": 5,
                "totalTokenCount": 15
            }
        }"#;
        let result = parse_response(raw).unwrap();
        assert_eq!(result.content, "Hello, I am Gemini!");
        assert_eq!(result.usage.input_tokens, 10);
        assert_eq!(result.usage.output_tokens, 5);
    }

    #[test]
    fn test_parse_response_error() {
        let raw = r#"{"error": "something went wrong"}"#;
        let result = parse_response(raw);
        // 应该返回空内容而不是错误
        assert_eq!(result.unwrap().content, "");
    }
}
