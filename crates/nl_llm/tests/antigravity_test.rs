use nl_llm::prompt_ast::{PromptAst, PromptNode};
use nl_llm::provider::antigravity::{AntigravityConfig, AntigravityProvider};
use std::path::PathBuf;

#[tokio::test]
#[ignore] // Requires manual login flow interaction or valid token file
async fn test_antigravity_compile_request() {
    let home = std::env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string());
    let token_path = PathBuf::from(home).join(".nl_llm").join("antigravity_token.json");

    let provider = AntigravityProvider::new(AntigravityConfig {
        model: "gemini-2.5-flash".to_string(),
        token_path,
    });

    let ast = PromptAst::new()
        .push(PromptNode::User("Hello from Antigravity test!".to_string()));
    let body = provider.compile_request(&ast);
    
    println!("Compiled Request: {}", serde_json::to_string_pretty(&body).unwrap());
    
    // Basic assertions
    assert_eq!(body["model"], "gemini-2.5-flash");
    assert!(body["request"]["contents"].is_array());
}

#[tokio::test]
#[ignore]
async fn test_antigravity_auth_headers() {
    let provider = AntigravityProvider::default_provider();
    
    // This will trigger login flow if no token exists, or refresh if expired.
    // It might block if user interaction is needed.
    let headers = provider.get_auth_headers().await;
    
    match headers {
        Ok(h) => {
            println!("Auth Headers: {:?}", h);
            assert!(h.contains_key("Authorization"));
            assert!(h.contains_key("Client-Metadata"));
        },
        Err(e) => {
            println!("Auth failed (expected if no token): {}", e);
        }
    }
}

#[tokio::test]
async fn test_antigravity_system_instruction() {
    let provider = AntigravityProvider::default_provider();

    let ast = PromptAst::new()
        .push(PromptNode::System("You are a helpful assistant.".to_string()))
        .push(PromptNode::User("Hello!".to_string()));
    let body = provider.compile_request(&ast);

    println!("Compiled Request: {}", serde_json::to_string_pretty(&body).unwrap());

    // systemInstruction should be present with system message
    let sys_instr = &body["request"]["systemInstruction"];
    assert!(sys_instr.is_object(), "systemInstruction should exist");
    assert!(sys_instr["parts"].is_array(), "systemInstruction.parts should be array");
    assert_eq!(
        sys_instr["parts"][0]["text"],
        "You are a helpful assistant."
    );

    // user message should NOT contain the system prefix
    let contents = &body["request"]["contents"];
    assert!(contents.is_array());
    let first_content = &contents[0];
    assert_eq!(first_content["role"], "user");
    assert_eq!(first_content["parts"][0]["text"], "Hello!");

    // Required fields should exist
    assert_eq!(body["model"], "gemini-2.5-flash");
    assert_eq!(body["userAgent"], "antigravity");
    assert_eq!(body["requestType"], "agent");
    assert!(body["project"].is_string());
    assert!(body["requestId"].as_str().unwrap().starts_with("agent-"));
    assert!(body["request"]["sessionId"].is_string());
}
