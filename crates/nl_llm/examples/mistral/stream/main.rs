use anyhow::Result;
use futures::StreamExt;
use nl_llm::{LlmClient, PrimitiveRequest};
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<()> {
    let api_key = std::env::var("MISTRAL_API_KEY").expect("MISTRAL_API_KEY 环境变量未设置");

    let client = LlmClient::from_preset("mistral")
        .expect("找不到 Mistral 预设")
        .with_api_key(api_key)
        .build();

    let model = "open-mistral-7b";

    let prompt = "Write a short 3 paragraph story about a brave knight.";
    println!("Streaming from Mistral...\n");

    let req = PrimitiveRequest::single_user_message(prompt).with_model(model);

    let mut stream = client.stream(&req).await?;

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(token) => {
                print!("{}", token.content);
                io::stdout().flush()?;
            }
            Err(e) => eprintln!("\nError during streaming: {:?}", e),
        }
    }

    println!("\n\nStream finished.");

    Ok(())
}
