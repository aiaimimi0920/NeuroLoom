//! Claude Provider 集成测试
//!
//! 测试 ClaudeProvider 的 compile() 输出

use nl_llm_new::primitive::{PrimitiveRequest, PrimitiveMessage};
use nl_llm_new::provider::claude::config::ClaudeConfig;
use nl_llm_new::provider::claude::provider::ClaudeProvider;
use nl_llm_new::provider::LlmProvider;

fn dummy_claude_provider() -> ClaudeProvider {
    ClaudeProvider::new(ClaudeConfig::new(
        "sk-ant-test-key".to_string(),
        "claude-sonnet-4-20250514".to_string(),
    ))
}

#[test]
fn test_claude_compile_basic() {
    let provider = dummy_claude_provider();
    let primitive = PrimitiveRequest::single_user_message("Hello")
        .with_max_tokens(1024);
    let body = provider.compile(&primitive);

    // Claude format: { "model": "...", "messages": [...], "max_tokens": N }
    assert!(body.get("model").is_some(), "should have 'model' field");
    assert!(body.get("messages").is_some(), "should have 'messages' field");
    assert!(body.get("max_tokens").is_some(), "should have 'max_tokens' field");

    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["role"], "user");
}

#[test]
fn test_claude_compile_system() {
    let provider = dummy_claude_provider();
    let primitive = PrimitiveRequest::with_system_and_user(
        "You are a poet.",
        "Write a haiku",
    ).with_max_tokens(256);
    let body = provider.compile(&primitive);

    // Claude puts system as an array of content blocks
    assert!(body.get("system").is_some(), "should have 'system' field");
    let system = body["system"].as_array().unwrap();
    assert_eq!(system[0]["text"], "You are a poet.");
}

#[test]
fn test_claude_compile_multi_turn() {
    let provider = dummy_claude_provider();
    let primitive = PrimitiveRequest::default()
        .with_message(PrimitiveMessage::user("Hello"))
        .with_message(PrimitiveMessage::assistant("Hi!"))
        .with_message(PrimitiveMessage::user("Goodbye"));
    let body = provider.compile(&primitive);

    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0]["role"], "user");
    assert_eq!(messages[1]["role"], "assistant");
    assert_eq!(messages[2]["role"], "user");
}

#[test]
fn test_claude_provider_id() {
    let provider = dummy_claude_provider();
    assert_eq!(provider.id(), "claude");
}
