//! azure_openai 平台测试 - auth
//!
//! 运行方式: cargo run --example azure_openai_auth
//! 或直接运行: test.bat

use anyhow::Result;
use nl_llm_v2::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let api_key = std::env::var("AZURE_OPENAI_API_KEY")
        .ok()
        .or_else(|| args.get(1).cloned())
        .unwrap_or_else(|| "dummy_credential".to_string());

    let client = LlmClient::from_preset("azure_openai")
        .expect("Preset should exist")
        .with_api_key(api_key)
        .build();

    let prompt = args.get(2).cloned().unwrap_or_else(|| "Hello!".to_string());

    let mut req = PrimitiveRequest::single_user_message(&prompt).with_model("unknown");

    println!("用户: {}\n", prompt);
    println!("AI:");

    let resp = client.complete(&req).await?;
    println!("{}", resp.content);

    Ok(())
}
