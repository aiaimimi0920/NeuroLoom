//! aihubmix 平台测试 - stream
//!
//! 运行方式: cargo run --example aihubmix_stream
//! 或直接运行: test.bat

use nl_llm_v2::{LlmClient, PrimitiveRequest};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let api_key = std::env::var("AIHUBMIX_API_KEY").ok()
        .or_else(|| args.get(1).cloned())
        .unwrap_or_else(|| "dummy_credential".to_string());

    let client = LlmClient::from_preset("aihubmix")
        .expect("Preset should exist")
        .with_api_key(api_key)
        .build();

    let prompt = args.get(2).cloned()
        .unwrap_or_else(|| "Hello!".to_string());

    let mut req = PrimitiveRequest::single_user_message(&prompt)
        .with_model("unknown");
    req.stream = true;

    println!("用户: {}\n", prompt);
    println!("AI (Stream):");

    let mut stream = client.stream(&req).await?;
    use tokio_stream::StreamExt;
    while let Some(chunk) = stream.next().await {
        if let Ok(c) = chunk {
            print!("{}", c.content);
        }
    }
    println!();
    Ok(())
}
