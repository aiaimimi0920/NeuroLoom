use anyhow::Result;
use tokio_stream::StreamExt;
use nl_llm_v2::{LlmClient, PrimitiveRequest};
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<()> {
    let api_key = std::env::var("JINA_API_KEY")
        .expect("JINA_API_KEY 环境变量未设置");

    let client = LlmClient::from_preset("jina")
        .expect("找不到 Jina 预设")
        .with_api_key(api_key)
        .build();

    let model = "jina-embeddings-v3"; // Or a chat model if Jina provides one
    
    println!("Streaming from Jina...");
    
    let req = PrimitiveRequest::single_user_message("给我讲一个短笑话")
        .with_model(model);
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

    println!("\nStream complete.");
    Ok(())
}
