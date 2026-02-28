use anyhow::Result;
use nl_llm::{LlmClient, PrimitiveRequest};
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let api_key = std::env::var("CUSTOM_API_KEY")
        .ok()
        .or_else(|| args.get(1).cloned())
        .unwrap_or_else(|| "sk-dummy-custom-key".to_string());

    let client = LlmClient::from_preset("custom")
        .expect("找不到 custom 预设")
        .with_api_key(api_key)
        .build();

    let default_prompt = "Write a short poem about coding.".to_string();
    let prompt = args.get(2).cloned().unwrap_or(default_prompt);

    let model = args
        .get(3)
        .cloned()
        .unwrap_or_else(|| "gpt-3.5-turbo".to_string());

    let req = PrimitiveRequest::single_user_message(&prompt).with_model(&model);

    println!("User: {}", prompt);
    println!("Model Request: {}", model);
    println!("Streaming Custom AI Output...\n");

    let mut stream = client.stream(&req).await?;

    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                use std::io::{self, Write};
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
