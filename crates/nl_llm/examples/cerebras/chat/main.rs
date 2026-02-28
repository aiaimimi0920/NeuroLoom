//! CereBras AI 平台测试 - chat
//!
//! 运行方式: cargo run --example cerebras_chat
//! 或直接运行: test.bat

use anyhow::Result;
use nl_llm::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let api_key = std::env::var("CEREBRAS_API_KEY")
        .ok()
        .or_else(|| args.get(1).cloned())
        .unwrap_or_else(|| "dummy_credential".to_string());

    let client = LlmClient::from_preset("cerebras")
        .expect("Preset should exist")
        .with_api_key(api_key)
        .build();

    let prompt = args.get(2).cloned().unwrap_or_else(|| "Hello! Tell me about yourself.".to_string());

    let req = PrimitiveRequest::single_user_message(&prompt).with_model("llama3.1-8b");

    println!("用户: {}\n", prompt);
    println!("AI:");

    let resp = client.complete(&req).await?;
    println!("{}", resp.content);

    Ok(())
}
