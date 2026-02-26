use anyhow::Result;
use nl_llm_v2::{LlmClient, PrimitiveRequest};
use std::io::{self, Write};
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    let api_key = std::env::var("MOKA_API_KEY")
        .expect("MOKA_API_KEY 环境变量未设置");

    // Initialize MokaAI preset client
    let client = LlmClient::from_preset("moka")
        .expect("找不到 moka 预设")
        .with_api_key(api_key)
        .build();

    println!("Streaming from MokaAI...");

    let req = PrimitiveRequest::single_user_message("解释一下光电效应。");
    
    let mut stream = client.stream(&req).await?;

    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                print!("{}", chunk.content);
                io::stdout().flush().unwrap();
            }
            Err(e) => {
                eprintln!("\n流读取错误: {:?}", e);
                break;
            }
        }
    }

    println!("\n\nStream complete.");
    Ok(())
}
