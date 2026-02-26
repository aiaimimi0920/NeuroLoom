use anyhow::Result;
use futures::StreamExt;
use nl_llm_v2::{LlmClient, PrimitiveRequest};
use tokio::io::{stdout, AsyncWriteExt};

#[tokio::main]
async fn main() -> Result<()> {
    let api_key = std::env::var("FASTGPT_API_KEY").expect("FASTGPT_API_KEY 环境变量未设置");

    let client = LlmClient::from_preset("fastgpt")
        .expect("找不到 FastGPT 预设")
        .with_api_key(api_key)
        .build();

    let prompt = "Write a short poem about coding.";
    println!("Sending stream request to FastGPT...\n");

    let req = PrimitiveRequest::single_user_message(prompt).with_model("fastgpt-default");

    let mut stream = client.stream(&req).await?;

    let mut out = stdout();
    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                out.write_all(chunk.content.as_bytes()).await?;
                out.flush().await?;
            }
            Err(e) => {
                eprintln!("\nError during streaming: {}", e);
                break;
            }
        }
    }
    println!("\n\nStream complete.");

    Ok(())
}
