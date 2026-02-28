//! xai 平台测试 - models
//!
//! 运行方式: cargo run --example xai_models
//! 或直接运行: test.bat

use anyhow::Result;
use nl_llm::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let api_key = std::env::var("XAI_API_KEY")
        .ok()
        .or_else(|| args.get(1).cloned())
        .unwrap_or_else(|| "dummy_credential".to_string());

    let client = LlmClient::from_preset("xai")
        .expect("Preset should exist")
        .with_api_key(api_key)
        .build();

    let prompt = args
        .get(2)
        .cloned()
        .unwrap_or_else(|| "Testing. Just say hi and hello world and nothing else.".to_string());

    // Just tests resolving a different model
    let mut req = PrimitiveRequest::single_user_message(&prompt).with_model("grok-2-latest");
    req.system = Some("You are a test assistant.".to_string());

    println!("用户: {}\n", prompt);
    println!("AI:");

    let resp = client.complete(&req).await?;
    println!("{}", resp.content);

    Ok(())
}
