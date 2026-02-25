//! AiHubMix 流式输出

use futures::StreamExt;
use nl_llm_v2::{LlmClient, PrimitiveRequest};
use std::io::Write;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("AIHUBMIX_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: aihubmix_stream <API_KEY> [prompt]");
            eprintln!("或设置 AIHUBMIX_API_KEY 环境变量");
            std::process::exit(1);
        });

    let prompt = std::env::args().nth(2).unwrap_or_else(|| "写一段简单的快速排序，不要解释".to_string());

    let client = LlmClient::from_preset("aihubmix")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    println!("========================================");
    println!("  AiHubMix 流式输出");
    println!("========================================\n");
    println!("模型: gpt-4o-free (1M context)");
    println!("用户: {}\n", prompt);
    print!("AI: ");
    std::io::stdout().flush()?;

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
