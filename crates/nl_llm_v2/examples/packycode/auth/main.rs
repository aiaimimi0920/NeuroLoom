//! packycode 平台测试 - auth
//!
//! 运行方式: cargo run --example packycode_auth
//! 或直接运行: test.bat

use nl_llm_v2::{LlmClient, PrimitiveRequest};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let api_key = std::env::var("PACKYCODE_API_KEY").ok()
        .or_else(|| args.get(1).cloned())
        .unwrap_or_else(|| "dummy_credential".to_string());

    let client = LlmClient::from_preset("packycode")
        .expect("Preset should exist")
        .with_api_key(api_key)
        .build();

    let prompt = args.get(2).cloned()
        .unwrap_or_else(|| "Hello!".to_string());

    let mut req = PrimitiveRequest::single_user_message(&prompt)
        .with_model("unknown");

    println!("用户: {}\n", prompt);
    println!("AI:");

    let resp = client.complete(&req).await?;
    println!("{}", resp.content);

    Ok(())
}
