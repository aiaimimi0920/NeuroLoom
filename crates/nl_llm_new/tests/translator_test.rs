//! 转换器管道集成测试
//!
//! 测试 PrimitiveRequest 与各种 Provider 编译器的交互

use nl_llm_new::primitive::{PrimitiveRequest, PrimitiveContent, PrimitiveTool};
use nl_llm_new::provider::gemini::GoogleAIStudioProvider;
use nl_llm_new::provider::vertex::VertexProvider;
use nl_llm_new::provider::claude::provider::ClaudeProvider;
use nl_llm_new::provider::claude::config::ClaudeConfig;
use nl_llm_new::provider::openai::provider::OpenAIProvider;
use nl_llm_new::provider::openai::config::OpenAIConfig;
use nl_llm_new::provider::LlmProvider;

/// 测试同一个 PrimitiveRequest 编译到多种格式
#[test]
fn test_cross_provider_compile() {
    let primitive = PrimitiveRequest::with_system_and_user(
        "You are a helpful coding assistant.",
        "Write a Rust hello world",
    ).with_max_tokens(2048);

    // Gemini
    let vertex = VertexProvider::from_service_account(
        r#"{"project_id":"test","client_email":"t@t.iam.gserviceaccount.com","private_key":"-----BEGIN PRIVATE KEY-----\ntest\n-----END PRIVATE KEY-----","private_key_id":""}"#.to_string(),
        "gemini-2.5-flash".to_string(), None,
    );
    let gemini_body = vertex.compile(&primitive);
    assert!(gemini_body.get("contents").is_some());
    assert!(gemini_body.get("systemInstruction").is_some());

    // Claude
    let claude = ClaudeProvider::new(ClaudeConfig::new("sk-test".to_string(), "claude-sonnet-4-20250514".to_string()));
    let claude_body = claude.compile(&primitive);
    assert!(claude_body.get("messages").is_some());
    assert!(claude_body.get("system").is_some());
    let claude_system = claude_body["system"].as_array().unwrap();
    assert_eq!(claude_system[0]["text"], "You are a helpful coding assistant.");

    // OpenAI
    let openai = OpenAIProvider::new(OpenAIConfig::new("sk-test".to_string(), "gpt-4o".to_string()));
    let openai_body = openai.compile(&primitive);
    assert!(openai_body.get("messages").is_some());
    let msgs = openai_body["messages"].as_array().unwrap();
    assert_eq!(msgs[0]["role"], "system");
}

/// 测试工具定义的编译
#[test]
fn test_compile_with_tools() {
    let tool = PrimitiveTool {
        name: "get_weather".to_string(),
        description: Some("Get current weather for a location".to_string()),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "City name"
                }
            },
            "required": ["location"]
        }),
    };

    let primitive = PrimitiveRequest::single_user_message("What's the weather in Tokyo?")
        .with_tool(tool);

    // Each provider should include tools in their compiled body
    let vertex = VertexProvider::from_service_account(
        r#"{"project_id":"test","client_email":"t@t.iam.gserviceaccount.com","private_key":"-----BEGIN PRIVATE KEY-----\ntest\n-----END PRIVATE KEY-----","private_key_id":""}"#.to_string(),
        "gemini-2.5-flash".to_string(), None,
    );
    let gemini_body = vertex.compile(&primitive);
    // Gemini uses "tools" with "functionDeclarations"
    assert!(gemini_body.get("tools").is_some(), "Gemini should have tools");

    let claude = ClaudeProvider::new(ClaudeConfig::new("sk-test".to_string(), "claude-sonnet-4-20250514".to_string()));
    let claude_body = claude.compile(&primitive);
    assert!(claude_body.get("tools").is_some(), "Claude should have tools");

    let openai = OpenAIProvider::new(OpenAIConfig::new("sk-test".to_string(), "gpt-4o".to_string()));
    let openai_body = openai.compile(&primitive);
    assert!(openai_body.get("tools").is_some(), "OpenAI should have tools");
}

/// 测试 PrimitiveRequest 便捷构造函数
#[test]
fn test_primitive_convenience_constructors() {
    let req1 = PrimitiveRequest::single_user_message("Hello");
    assert_eq!(req1.messages.len(), 1);
    assert!(req1.system.is_none());

    let req2 = PrimitiveRequest::with_system_and_user("System prompt", "User prompt");
    assert_eq!(req2.messages.len(), 1);
    assert_eq!(req2.system.as_deref(), Some("System prompt"));

    // Verify the content of the user message
    if let PrimitiveContent::Text { text } = &req2.messages[0].content[0] {
        assert_eq!(text, "User prompt");
    } else {
        panic!("Expected text content");
    }
}
