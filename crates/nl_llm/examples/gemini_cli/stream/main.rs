//! gemini_cli 平台测试 - stream
//!
//! 运行方式: cargo run --example gemini_cli_stream
//! 或直接运行: test.bat

use anyhow::Result;
use nl_llm::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let api_key = std::env::var("GEMINI_CLI_API_KEY")
        .ok()
        .or_else(|| args.get(1).cloned())
        .unwrap_or_else(|| "dummy_credential".to_string());

    let client = LlmClient::from_preset("gemini_cli")
        .expect("Preset should exist")
        .build();

    let prompt = args.get(2).cloned().unwrap_or_else(|| "Hello!".to_string());

    let mut req = PrimitiveRequest::single_user_message(&prompt).with_model("gemini-2.5-pro");
    req.stream = true;

    println!("用户: {}\n", prompt);
    println!("AI (Stream):");

    let mut stream = client.stream(&req).await?;
    use tokio_stream::StreamExt;
    while let Some(chunk) = stream.next().await {
        if let Ok(c) = chunk {
            print!("{}", c.content);
        }
    }
    println!();
    Ok(())
}
