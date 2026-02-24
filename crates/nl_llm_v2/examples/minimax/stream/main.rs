//! MiniMax (en) 流式输出示例

use futures::StreamExt;
use nl_llm_v2::{LlmClient, PrimitiveRequest};
use std::io::Write;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("MINIMAX_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| "invalid_dummy_key_for_testing".to_string());

    let client = LlmClient::from_preset("minimax")
        .expect("Preset should exist")
        .with_api_key(api_key)
        .build();

    let prompt = std::env::args().nth(2).unwrap_or_else(|| "写一段简单的快速排序，不要解释".to_string());

    println!("========================================");
    println!("  MiniMax 流式输出");
    println!("========================================\n");
    println!("模型: MiniMax-M2.5 (200K context)");
    println!("用户: {}\n", prompt);
    print!("AI: \n");

    let req = PrimitiveRequest::single_user_message(&prompt);
    let mut stream = client.stream(&req).await?;
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(c) => { print!("{}", c.content); std::io::stdout().flush()?; }
            Err(e) => { eprintln!("\n读取流错误: {}", e); break; }
        }
    }
    println!("\n");
    Ok(())
}
