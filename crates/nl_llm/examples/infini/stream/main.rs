//! 无问芯穹 (Infinigence AI) 平台测试 - stream
//!
//! 运行方式: cargo run --example infini_stream
//! 或直接运行: test.bat

use anyhow::Result;
use nl_llm::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let api_key = std::env::var("INFINI_API_KEY")
        .ok()
        .or_else(|| args.get(1).cloned())
        .unwrap_or_else(|| "dummy_credential".to_string());

    let client = LlmClient::from_preset("infini")
        .expect("Preset should exist")
        .with_api_key(api_key)
        .build();

    let prompt = args
        .get(2)
        .cloned()
        .unwrap_or_else(|| "请写一首关于星空的短诗。".to_string());
    let model = args
        .get(3)
        .cloned()
        .unwrap_or_else(|| "qwen2.5-72b-instruct".to_string());

    let mut req = PrimitiveRequest::single_user_message(&prompt).with_model(&model);
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
