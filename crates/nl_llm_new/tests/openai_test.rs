//! OpenAI Provider 集成测试
//!
//! 测试 OpenAIProvider 的 compile() 输出

use nl_llm_new::primitive::{PrimitiveRequest, PrimitiveMessage};
use nl_llm_new::provider::openai::config::OpenAIConfig;
use nl_llm_new::provider::openai::provider::OpenAIProvider;
use nl_llm_new::provider::LlmProvider;

fn dummy_openai_provider() -> OpenAIProvider {
    OpenAIProvider::new(OpenAIConfig::new(
        "sk-test-key".to_string(),
        "gpt-4o".to_string(),
    ))
}

#[test]
fn test_openai_compile_basic() {
    let provider = dummy_openai_provider();
    let primitive = PrimitiveRequest::single_user_message("Hello");
    let body = provider.compile(&primitive);

    // OpenAI format: { "model": "...", "messages": [...] }
    assert!(body.get("model").is_some(), "should have 'model' field");
    assert!(body.get("messages").is_some(), "should have 'messages' field");

    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["role"], "user");
    assert!(messages[0].get("content").is_some());
}

#[test]
fn test_openai_compile_system() {
    let provider = dummy_openai_provider();
    let primitive = PrimitiveRequest::with_system_and_user(
        "You are a helpful assistant.",
        "Hello",
    );
    let body = provider.compile(&primitive);

    let messages = body["messages"].as_array().unwrap();
    // System message should be first
    assert_eq!(messages[0]["role"], "system");
    assert_eq!(messages[1]["role"], "user");
}

#[test]
fn test_openai_compile_multi_turn() {
    let provider = dummy_openai_provider();
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
fn test_openai_provider_id() {
    let provider = dummy_openai_provider();
    assert_eq!(provider.id(), "openai");
}
