use anyhow::Result;
use nl_llm::{LlmClient, PrimitiveRequest};
use std::io::{self, Write};
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    let api_key = std::env::var("AIPROXY_API_KEY").expect("AIPROXY_API_KEY 环境变量未设置");

    let client = LlmClient::from_preset("aiproxy")
        .expect("找不到 aiproxy 预设")
        .with_api_key(api_key)
        .build();

    println!("Streaming from AI Proxy...");

    let req = PrimitiveRequest::single_user_message("讲述一下量子力学的基本原理，带分词。");

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
