use anyhow::Result;
use futures::StreamExt;
use nl_llm_v2::presets;
use nl_llm_v2::provider::traits::LlmClient;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let api_key = std::env::var("DIFY_API_KEY")
        .expect("DIFY_API_KEY 环境变量未设置");

    let client = presets::REGISTRY
        .get_builder("dify")
        .expect("找不到 Dify 预设")
        .auth(nl_llm_v2::site::Auth::api_key(api_key))
        .build()?;

    println!("Streaming from Dify...");
    
    let mut stream = client.stream("dify", "给我讲一个短笑话").await?;

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

    println!("\nStream complete.");
    Ok(())
}
