use nl_llm::prompt_ast::{PromptAst, PromptNode};
use nl_llm::provider::{IFlowConfig, IFlowProvider};
use std::env;

#[tokio::test]
#[ignore] // 默认忽略，因为需要真实 Cookie
async fn test_iflow_integration() {
    // 从环境变量获取 Cookie
    let cookie = env::var("IFLOW_COOKIE").expect("IFLOW_COOKIE not set");

    let config = IFlowConfig {
        cookie,
        ..Default::default()
    };

    let mut provider = IFlowProvider::new(config);

    // 1. 测试获取 API Key
    let api_key = provider.refresh_api_key().await.expect("refresh api key failed");
    println!("Got API Key: {}", api_key);
    assert!(!api_key.is_empty());

    // 2. 测试简单对话
    let ast = PromptAst::new()
        .push(PromptNode::User("你好，请回复'pong'".to_string()));

    let response = provider.complete(&ast).await.expect("completion failed");
    println!("Response: {}", response);
    assert!(!response.is_empty());
}
