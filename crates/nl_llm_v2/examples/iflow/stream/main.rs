//! iFlow 平台测试 - 流式输出
//!
//! 运行方式: cargo run -p nl_llm_v2 --example iflow_stream

use nl_llm_v2::{LlmClient, PrimitiveRequest};
use anyhow::Result;
use futures::StreamExt;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let cookie = args.get(1).cloned()
        .or_else(|| std::env::var("IFLOW_COOKIE").ok())
        .or_else(|| read_cookie_from_config())
        .expect("请提供 iFlow Cookie");

    let prompt = args.get(2).cloned()
        .unwrap_or_else(|| "用三句话介绍一下Rust语言".to_string());

    let model = args.get(3).cloned()
        .unwrap_or_else(|| "qwen3-max".to_string());

    let client = LlmClient::from_preset("iflow")
        .expect("iflow preset should exist")
        .with_cookie(&cookie)
        .build();

    let req = PrimitiveRequest::single_user_message(&prompt)
        .with_model(&model);

    println!("用户: {}\n", prompt);
    println!("AI ({}，流式):", model);

    let mut stream = client.stream(&req).await?;
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(c) => print!("{}", c.content),
            Err(e) => {
                eprintln!("\n流式错误: {}", e);
                break;
            }
        }
    }
    println!();

    Ok(())
}

fn read_cookie_from_config() -> Option<String> {
    let paths = [
        "crates/nl_llm_v2/examples/iflow/iflow_config.txt",
        "examples/iflow/iflow_config.txt",
    ];
    for p in &paths {
        if let Some(cookie) = try_read_config(Path::new(p)) {
            return Some(cookie);
        }
    }
    None
}

fn try_read_config(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("BXAuth=") {
            return Some(line.to_string());
        }
    }
    None
}
