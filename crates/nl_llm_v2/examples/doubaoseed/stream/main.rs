//! DouBaoSeed 流��输出
//!
//! 运行方式:
//!   方式1: cargo run -p nl_llm_v2 --example doubaoseed_stream -- <api_key> [prompt]
//!   方式2: 使用 test.bat（自动读取 .env.local 中的密钥）

use futures::StreamExt;
use nl_llm_v2::{LlmClient, PrimitiveRequest};
use std::io::Write;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("DOUBAOSEED_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: doubaoseed_stream <API_KEY> [prompt]");
            eprintln!("或设置 DOUBAOSEED_API_KEY 环境变量");
            std::process::exit(1);
        });

    let prompt = std::env::args().nth(2).unwrap_or_else(|| "写一段简单的冒泡排序，不要解释".to_string());

    let client = LlmClient::from_preset("doubaoseed")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .with_concurrency()
        .build();

    println!("========================================");
    println!("  DouBaoSeed (豆包) 流式输出");
    println!("========================================\n");
    println!("模型: {}", client.resolve_model("doubao"));
    println!("用户: {}\n", prompt);
    print!("AI: ");
    std::io::stdout().flush()?;

    let req = PrimitiveRequest::single_user_message(&prompt)
        .with_model("doubao");
    let mut stream = client.stream(&req).await?;
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(c) => { print!("{}", c.content); std::io::stdout().flush()?; }
            Err(e) => { eprintln!("\n读取流错误: {}", e); break; }
        }
    }
    println!("\n");

    // 显示并发状态
    if let Some(snapshot) = client.concurrency_snapshot() {
        println!("并发状态: 成功 {}, 平均延迟 {:?}ms",
            snapshot.success_count, snapshot.avg_latency_ms);
    }

    Ok(())
}
