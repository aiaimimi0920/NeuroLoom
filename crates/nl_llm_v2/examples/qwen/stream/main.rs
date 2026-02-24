//! Qwen OAuth 流式输出测试

use nl_llm_v2::{LlmClient, PrimitiveRequest};
use anyhow::Result;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let cache_path = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("qwen")
        .join("token.json");

    let client = LlmClient::from_preset("qwen")
        .expect("Preset should exist")
        .with_qwen_oauth(&cache_path)
        .build();

    let prompt = args.get(1).cloned()
        .unwrap_or_else(|| "Hello! Tell me a short story.".to_string());

    let req = PrimitiveRequest::single_user_message(&prompt)
        .with_model("qwen3-coder-plus");

    println!("用户: {}\n", prompt);
    println!("AI (Stream):");

    let mut stream = client.stream(&req).await?;

    use std::io::Write;
    while let Some(chunk_res) = stream.next().await {
        match chunk_res {
            Ok(chunk) => {
                print!("{}", chunk.content);
                std::io::stdout().flush().unwrap();
            }
            Err(e) => {
                eprintln!("\n流读取中断: {}", e);
                break;
            }
        }
    }
    println!();

    Ok(())
}
