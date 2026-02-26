use anyhow::Result;
use futures::StreamExt;
use nl_llm_v2::{LlmClient, PrimitiveRequest};
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<()> {
    let api_key = std::env::var("DIFY_API_KEY")
        .expect("DIFY_API_KEY 环境变量未设置");

    // The API URL inside the proxy might require the Dify user
    let client = LlmClient::from_preset("dify")
        .expect("找不到 Dify 预设")
        .with_api_key(api_key)
        .build();

    println!("Streaming from Dify...");
    
    let prompt = "给我讲一个短笑话";
    let req = PrimitiveRequest::single_user_message(prompt)
        .with_model("dify");
        
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
