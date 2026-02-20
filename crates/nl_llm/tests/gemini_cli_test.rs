use std::path::PathBuf;
use nl_llm::provider::gemini_cli::{GeminiCliConfig, GeminiCliProvider};
use nl_llm::prompt_ast::{PromptAst, PromptNode};

#[tokio::test]
async fn test_gemini_cli_compile_request() {
    let home = std::env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string());
    let token_path = PathBuf::from(home).join(".nl_llm").join("gemini_cli_token.json");

    let provider = GeminiCliProvider::new(GeminiCliConfig {
        model: "gemini-2.5-flash".to_string(),
        token_path,
    });

    let ast = PromptAst::new()
        .push(PromptNode::User("Hello, who are you?".to_string()));

    let body = provider.compile_request(&ast);
    let body_str = serde_json::to_string_pretty(&body).unwrap();
    println!("Compiled request:\n{}", body_str);

    // 验证 Gemini CLI 特有字段
    assert_eq!(body["userAgent"], "gemini-cli");
    assert_eq!(body["requestType"], "agent");
    assert!(body["request"]["contents"].is_array());
}

#[tokio::test]
#[ignore]
async fn test_gemini_cli_auth_headers() {
    let provider = GeminiCliProvider::default_provider();
    
    match provider.ensure_authenticated().await {
        Ok(token) => {
            println!("Access token (first 20 chars): {}...", &token[..20.min(token.len())]);
            let headers = provider.get_auth_headers().await.unwrap();
            println!("Auth headers: {:?}", headers);
            
            // 验证 Client-Metadata 是逗号分隔格式
            let metadata = headers.get("Client-Metadata").unwrap().to_str().unwrap();
            assert!(metadata.contains("ideType="));
            assert!(metadata.contains(","));
            assert!(!metadata.contains("{"));  // 不是 JSON 格式
        }
        Err(e) => {
            println!("Auth failed (expected if no token): {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_gemini_cli_system_instruction() {
    let provider = GeminiCliProvider::default_provider();

    let ast = PromptAst::new()
        .push(PromptNode::System("You are a helpful assistant.".to_string()))
        .push(PromptNode::User("Hi!".to_string()));

    let body = provider.compile_request(&ast);
    let body_str = serde_json::to_string_pretty(&body).unwrap();
    println!("Compiled request with system instruction:\n{}", body_str);

    // 验证 systemInstruction 存在
    let sys = &body["request"]["systemInstruction"];
    assert!(sys.is_object(), "systemInstruction should be an object");
    assert!(sys["parts"].is_array(), "systemInstruction.parts should be an array");

    let parts = sys["parts"].as_array().unwrap();
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0]["text"], "You are a helpful assistant.");

    // 验证 contents 中没有 system role
    let contents = body["request"]["contents"].as_array().unwrap();
    for c in contents {
        assert_ne!(c["role"], "system", "system role should not appear in contents");
    }
}
