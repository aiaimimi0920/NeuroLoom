//! TokenFlux 平台流式输出测试
//!
//! 运行方式: cargo run --example tokenflux_stream
//! 或直接运行: test.bat

use anyhow::Result;
use nl_llm_v2::{LlmClient, PrimitiveRequest};
use std::io::{stdout, Write};
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let api_key = std::env::var("TOKENFLUX_API_KEY")
        .unwrap_or_else(|_| args.get(1).cloned().expect("需要提供 API Key"));

    let client = LlmClient::from_preset("tokenflux")
        .expect("Preset not found")
        .with_api_key(api_key)
        .build();

    let prompt = args
        .get(2)
        .cloned()
        .unwrap_or_else(|| "写一首50字左右关于春天的短诗。".to_string());

    let mut req = PrimitiveRequest::single_user_message(&prompt).with_model("gpt-3.5-turbo");
    req.stream = true;

    println!("用户: {}\n", prompt);
    println!("AI:");

    let mut stream = client.stream(&req).await?;

    let mut total_chars = 0;
    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                print!("{}", chunk.content);
                stdout().flush()?;
                total_chars += chunk.content.chars().count();
            }
            Err(e) => {
                eprintln!("\n流式接收出错: {}", e);
                break;
            }
        }
    }

    println!("\n\n(完成，共 {} 字符)", total_chars);
    Ok(())
}
