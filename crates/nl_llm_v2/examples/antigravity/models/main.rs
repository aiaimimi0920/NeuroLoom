//! antigravity 平台测试 - models
//!
//! 运行方式: cargo run --example antigravity_models
//! 或直接运行: test.bat

use anyhow::Result;
use nl_llm_v2::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let api_key = std::env::var("ANTIGRAVITY_API_KEY")
        .ok()
        .or_else(|| args.get(1).cloned())
        .unwrap_or_else(|| "dummy_credential".to_string());

    let client = LlmClient::from_preset("antigravity")
        .expect("Preset should exist")
        .build();

    let prompt = args.get(2).cloned().unwrap_or_else(|| "Hello!".to_string());

    let mut req = PrimitiveRequest::single_user_message(&prompt).with_model("gemini-2.5-pro");

    println!("用户: {}\n", prompt);
    println!("AI:");

    let resp = client.complete(&req).await?;
    println!("{}", resp.content);

    Ok(())
}
