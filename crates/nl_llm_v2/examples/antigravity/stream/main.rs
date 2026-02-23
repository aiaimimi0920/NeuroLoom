//! antigravity 平台测试 - stream
//!
//! 运行方式: cargo run --example antigravity_stream
//! 或直接运行: test.bat

use futures::StreamExt;
use nl_llm_v2::{LlmClient, PrimitiveRequest};
use anyhow::Result;
use std::io::Write;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let api_key = std::env::var("ANTIGRAVITY_API_KEY").ok()
        .or_else(|| args.get(1).cloned())
        .unwrap_or_else(|| "dummy_credential".to_string());

    let client = LlmClient::from_preset("antigravity")
        .expect("Preset should exist")
        .with_antigravity_oauth(api_key)
        .build();

    let prompt = args.get(2).cloned()
        .unwrap_or_else(|| "Hello! Tell me a long story.".to_string());

    let req = PrimitiveRequest::single_user_message(&prompt)
        .with_model("gemini-2.5-flash");

    println!("用户: {}\n", prompt);
    println!("AI:");

    let mut stream = client.stream(&req).await?;

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(c) => {
                print!("{}", c.content);
                let _ = std::io::stdout().flush();
            }
            Err(e) => {
                println!("\n[Stream Error] {}", e);
                break;
            }
        }
    }
    println!();

    Ok(())
}
