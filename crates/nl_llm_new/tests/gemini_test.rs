//! Gemini Provider 集成测试
//!
//! 测试 Vertex 和 GoogleAIStudio Provider 的 compile() 输出

use nl_llm_new::primitive::{PrimitiveRequest, PrimitiveMessage};
use nl_llm_new::provider::gemini::config::*;
use nl_llm_new::provider::gemini::provider::*;
use nl_llm_new::provider::LlmProvider;

fn dummy_vertex_provider() -> VertexProvider {
    let sa_json = r#"{"project_id":"test-proj","client_email":"test@test.iam.gserviceaccount.com","private_key":"-----BEGIN PRIVATE KEY-----\ntest\n-----END PRIVATE KEY-----","private_key_id":""}"#;
    VertexProvider::from_service_account(
        sa_json.to_string(),
        "gemini-2.5-flash".to_string(),
        None,
    )
}

#[test]
fn test_vertex_compile_basic() {
    let provider = dummy_vertex_provider();
    let primitive = PrimitiveRequest::single_user_message("Hello");
    let body = provider.compile(&primitive);

    // Gemini format: { "contents": [{ "role": "user", "parts": [...] }] }
    assert!(body.get("contents").is_some(), "should have 'contents' field");
    let contents = body["contents"].as_array().unwrap();
    assert_eq!(contents.len(), 1);
    assert_eq!(contents[0]["role"], "user");
    let parts = contents[0]["parts"].as_array().unwrap();
    assert!(parts[0].get("text").is_some());
    assert_eq!(parts[0]["text"], "Hello");
}

#[test]
fn test_vertex_compile_system_instruction() {
    let provider = dummy_vertex_provider();
    let primitive = PrimitiveRequest::with_system_and_user(
        "You are a helpful assistant.",
        "Hello",
    );
    let body = provider.compile(&primitive);

    // System instruction should be in "systemInstruction" (camelCase)
    assert!(body.get("systemInstruction").is_some(), "should have systemInstruction");
    let sys = &body["systemInstruction"];
    let parts = sys["parts"].as_array().unwrap();
    assert_eq!(parts[0]["text"], "You are a helpful assistant.");
}

#[test]
fn test_vertex_compile_multi_turn() {
    let provider = dummy_vertex_provider();
    let primitive = PrimitiveRequest::default()
        .with_message(PrimitiveMessage::user("Hello"))
        .with_message(PrimitiveMessage::assistant("Hi there!"))
        .with_message(PrimitiveMessage::user("How are you?"));
    let body = provider.compile(&primitive);

    let contents = body["contents"].as_array().unwrap();
    assert_eq!(contents.len(), 3);
    assert_eq!(contents[0]["role"], "user");
    assert_eq!(contents[1]["role"], "model"); // Gemini uses "model" not "assistant"
    assert_eq!(contents[2]["role"], "user");
}

#[test]
fn test_google_ai_studio_compile_basic() {
    let provider = GoogleAIStudioProvider::from_api_key(
        "AIzaSyTestKey".to_string(),
        "gemini-2.5-flash".to_string(),
    );
    let primitive = PrimitiveRequest::single_user_message("Test prompt");
    let body = provider.compile(&primitive);

    // Should produce same format as Vertex (shared compiler)
    assert!(body.get("contents").is_some());
    let contents = body["contents"].as_array().unwrap();
    assert_eq!(contents[0]["parts"][0]["text"], "Test prompt");
}

#[test]
fn test_provider_ids() {
    let vertex = dummy_vertex_provider();
    assert_eq!(vertex.id(), "vertex");

    let studio = GoogleAIStudioProvider::from_api_key(
        "test".to_string(), "gemini-2.5-flash".to_string(),
    );
    assert_eq!(studio.id(), "google_ai_studio");
}
