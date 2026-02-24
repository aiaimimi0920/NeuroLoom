//! SiliconFlow EN (国际版) 流式输出

use futures::StreamExt;
use nl_llm_v2::{LlmClient, PrimitiveRequest};
use std::io::Write;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("SILICONFLOW_EN_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: siliconflow_en_stream <API_KEY> [prompt]");
            eprintln!("或设置 SILICONFLOW_EN_API_KEY 环境变量");
            std::process::exit(1);
        });

    let prompt = std::env::args().nth(2).unwrap_or_else(|| "Write a simple quicksort, no explanation".to_string());

    let client = LlmClient::from_preset("siliconflow_en")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    println!("========================================");
    println!("  SiliconFlow EN (国际版) 流式输出");
    println!("========================================\n");
    println!("模型: deepseek-ai/DeepSeek-V3");
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
