//! Gemini 请求编译器
//!
//! 提供 `GeminiCompiler` 结构体，用于将 PrimitiveRequest 编译为 Gemini JSON 请求体
//!
//! 注意：此模块是对 `common::compile_gemini_request` 的薄封装，保持向后兼容

use crate::primitive::PrimitiveRequest;
use serde_json::Value;

/// Gemini 请求编译器
///
/// 将 PrimitiveRequest 编译为 Gemini/Vertex JSON 请求体
pub struct GeminiCompiler;

impl GeminiCompiler {
    /// 编译 PrimitiveRequest 为 Gemini JSON 请求体
    pub fn compile(&self, primitive: &PrimitiveRequest) -> Value {
        super::common::compile_gemini_request(primitive)
    }
}

// ── 测试 ─────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitive::PrimitiveMessage;

    #[test]
    fn test_compile_user_message() {
        let primitive = PrimitiveRequest {
            model: "gemini-2.5-flash".to_string(),
            messages: vec![PrimitiveMessage::user("Hello Gemini!")],
            ..Default::default()
        };
        let compiler = GeminiCompiler;
        let body = compiler.compile(&primitive);

        let contents = body["contents"].as_array().unwrap();
        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0]["role"], "user");
        assert_eq!(contents[0]["parts"][0]["text"], "Hello Gemini!");
    }

    #[test]
    fn test_compile_assistant_message() {
        let primitive = PrimitiveRequest {
            model: "gemini-2.5-flash".to_string(),
            messages: vec![
                PrimitiveMessage::user("Hi"),
                PrimitiveMessage::assistant("Hello!"),
            ],
            ..Default::default()
        };
        let compiler = GeminiCompiler;
        let body = compiler.compile(&primitive);

        let contents = body["contents"].as_array().unwrap();
        assert_eq!(contents.len(), 2);
        assert_eq!(contents[0]["role"], "user");
        assert_eq!(contents[1]["role"], "model"); // assistant -> model
    }
}
