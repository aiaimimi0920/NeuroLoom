//! dify 平台测试 - stream
//!
//! 运行方式: cargo run -p nl_llm_v2 --example dify_stream

use anyhow::Result;
use nl_llm_v2::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let api_key = std::env::var("DIFY_API_KEY")
        .ok()
        .or_else(|| args.get(1).cloned())
        .unwrap_or_else(|| "dummy_credential".to_string());

    let client = LlmClient::from_preset("dify")
        .expect("Preset should exist")
        .with_api_key(api_key)
        .build();

    let prompt = args
        .get(2)
        .cloned()
        .unwrap_or_else(|| "给我讲一个短笑话".to_string());

    let mut req = PrimitiveRequest::single_user_message(&prompt).with_model("dify");
    req.stream = true;
    // 演示 metadata.user_id -> dify body.user 推导逻辑
    req.metadata.user_id = Some("demo-user".into());

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
