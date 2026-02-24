//! Kimi OAuth 流式输出测试

use nl_llm_v2::{LlmClient, PrimitiveRequest};
use futures::StreamExt;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let cache_path = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("kimi")
        .join("token.json");

    let client = LlmClient::from_preset("kimi")
        .expect("Preset should exist")
        .with_kimi_oauth(&cache_path)
        .build();

    let prompt = std::env::args().nth(1)
        .unwrap_or_else(|| "Hello! Tell me a short story.".to_string());

    let req = PrimitiveRequest::single_user_message(&prompt)
        .with_model("kimi-k2");

    println!("用户: {}\n", prompt);
    println!("AI (Stream):");

    let mut stream = client.stream(&req).await?;
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(c) => print!("{}", c.content),
            Err(e) => eprintln!("\n流式错误: {}", e),
        }
    }
    println!();

    Ok(())
}
