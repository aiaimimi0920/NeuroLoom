//! xai 平台测试 - stream
//!
//! 运行方式: cargo run --example xai_stream
//! 或直接运行: test.bat

use anyhow::Result;
use nl_llm::{LlmClient, PrimitiveRequest};
use std::io::Write;
use tokio_stream::StreamExt;

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

    let mut req = PrimitiveRequest::single_user_message(&prompt).with_model("grok-4-latest");
    req.stream = true;
    req.system = Some("You are a test assistant.".to_string());
    req.parameters.temperature = Some(0.0);

    println!("用户: {}\n", prompt);
    println!("AI:");

    let mut stream = client.stream(&req).await?;

    while let Some(chunk_res) = stream.next().await {
        match chunk_res {
            Ok(chunk) => {
                print!("{}", chunk.content);
                std::io::stdout().flush()?;
            }
            Err(e) => {
                eprintln!("\nStream error: {}", e);
                break;
            }
        }
    }

    println!();

    Ok(())
}
