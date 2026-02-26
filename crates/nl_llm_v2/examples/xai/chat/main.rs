//! xai 平台测试 - chat
//!
//! 运行方式: cargo run --example xai_chat
//! 或直接运行: test.bat

use nl_llm_v2::{LlmClient, PrimitiveRequest};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let api_key = std::env::var("XAI_API_KEY").ok()
        .or_else(|| args.get(1).cloned())
        .unwrap_or_else(|| "dummy_credential".to_string());

    let client = LlmClient::from_preset("xai")
        .expect("Preset should exist")
        .with_api_key(api_key)
        .build();

    let prompt = args.get(2).cloned()
        .unwrap_or_else(|| "Testing. Just say hi and hello world and nothing else.".to_string());

    let mut req = PrimitiveRequest::single_user_message(&prompt)
        .with_model("grok-4-latest");
    req.system = Some("You are a test assistant.".to_string());
    // Grok docs indicate temperature can be 0.
    req.parameters.temperature = Some(0.0);

    println!("用户: {}\n", prompt);
    println!("AI:");

    let resp = client.complete(&req).await?;
    println!("{}", resp.content);

    Ok(())
}
