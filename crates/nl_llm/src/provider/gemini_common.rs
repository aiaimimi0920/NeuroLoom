//! Gemini 系列 Provider 公共代码
//!
//! 提供以下共享函数：
//! - `compile_gemini_request`: 将 PromptAst 编译为 Gemini JSON 请求体
//! - `parse_gemini_response`: 解析 Gemini 非流式响应
//! - `parse_gemini_sse_stream`: 解析 Gemini SSE 流式响应

use crate::prompt_ast::PromptAst;
use serde_json::Value;

/// 将 PromptAst 编译为 Gemini/Vertex JSON 请求体
///
/// Gemini native 格式：
/// - role: "user" | "model"（不用 "assistant"）
/// - parts: [{ "text": "..." }]
/// - systemInstruction: { "parts": [...] }
pub fn compile_gemini_request(ast: &PromptAst) -> Value {
    let openai_msgs = ast.to_openai_messages();

    let mut system_parts: Vec<Value> = Vec::new();
    let mut contents: Vec<Value> = Vec::new();

    for msg in &openai_msgs {
        let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("user");
        let text = msg.get("content").and_then(|c| c.as_str()).unwrap_or("");

        match role {
            "system" => {
                if !text.is_empty() {
                    system_parts.push(serde_json::json!({ "text": text }));
                }
            }
            "assistant" => {
                contents.push(serde_json::json!({
                    "role": "model",
                    "parts": [{ "text": text }]
                }));
            }
            _ => {
                contents.push(serde_json::json!({
                    "role": "user",
                    "parts": [{ "text": text }]
                }));
            }
        }
    }

    if contents.is_empty() && !system_parts.is_empty() {
        contents.push(serde_json::json!({
            "role": "user",
            "parts": [{ "text": "" }]
        }));
    }

    let mut body = serde_json::json!({ "contents": contents });

    if !system_parts.is_empty() {
        body["systemInstruction"] = serde_json::json!({ "parts": system_parts });
    }

    body
}

/// 解析 Gemini 非流式响应：candidates[0].content.parts[0].text
pub fn parse_gemini_response(raw: &str) -> crate::Result<String> {
    let json: Value = serde_json::from_str(raw).map_err(|e| {
        crate::NeuroLoomError::LlmProvider(format!(
            "gemini: generateContent decode response failed: {}",
            e
        ))
    })?;

    json.get("candidates")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("content"))
        .and_then(|c| c.get("parts"))
        .and_then(|p| p.get(0))
        .and_then(|p| p.get("text"))
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            crate::NeuroLoomError::LlmProvider(
                "gemini: unexpected generateContent response format".to_string(),
            )
        })
}

/// 解析 SSE 流，拼接所有 chunk 的 text
pub async fn parse_gemini_sse_stream(resp: reqwest::Response) -> crate::Result<String> {
    use futures::StreamExt;

    let mut stream = resp.bytes_stream();
    let mut buffer = String::new();
    let mut result = String::new();

    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(|e| {
            crate::NeuroLoomError::LlmProvider(format!("gemini: stream read error: {}", e))
        })?;
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
                        result.push_str(text);
                    }
                }
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompt_ast::PromptNode;

    #[test]
    fn test_compile_gemini_request_user_message() {
        let ast = PromptAst::new().push(PromptNode::User("Hello Gemini!".to_string()));
        let body = compile_gemini_request(&ast);

        let contents = body["contents"].as_array().unwrap();
        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0]["role"], "user");
        assert_eq!(contents[0]["parts"][0]["text"], "Hello Gemini!");
    }

    #[test]
    fn test_compile_gemini_request_system_message() {
        let ast = PromptAst::new()
            .push(PromptNode::System("You are a helpful AI.".to_string()))
            .push(PromptNode::User("Hello!".to_string()));
        let body = compile_gemini_request(&ast);

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
    fn test_compile_gemini_request_assistant_message() {
        let ast = PromptAst::new()
            .push(PromptNode::User("Hi".to_string()))
            .push(PromptNode::Assistant("Hello!".to_string()))
            .push(PromptNode::User("How are you?".to_string()));
        let body = compile_gemini_request(&ast);

        let contents = body["contents"].as_array().unwrap();
        assert_eq!(contents.len(), 3);
        assert_eq!(contents[0]["role"], "user");
        assert_eq!(contents[1]["role"], "model"); // assistant -> model
        assert_eq!(contents[2]["role"], "user");
    }

    #[test]
    fn test_parse_gemini_response() {
        let raw = r#"{
            "candidates": [{
                "content": {
                    "parts": [{ "text": "Hello, I am Gemini!" }],
                    "role": "model"
                }
            }]
        }"#;
        let result = parse_gemini_response(raw).unwrap();
        assert_eq!(result, "Hello, I am Gemini!");
    }

    #[test]
    fn test_parse_gemini_response_error() {
        let raw = r#"{"error": "something went wrong"}"#;
        let result = parse_gemini_response(raw);
        assert!(result.is_err());
    }
}
