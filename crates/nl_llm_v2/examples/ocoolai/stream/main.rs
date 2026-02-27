//! ocoolAI 平台流式输出测试
//!
//! 运行方式:
//! - 直接运行 test.bat
//! - 或: cargo run --example ocoolai_stream -- <api_key> [prompt]
//!
//! 环境变量:
//! - OCOOLAI_API_KEY: API Key（可选，可从 .env.local 读取）

use anyhow::Result;
use futures::StreamExt;
use nl_llm_v2::{LlmClient, PrimitiveRequest};

fn load_env_from_file() {
    let env_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("ocoolai")
        .join(".env.local");

    if env_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&env_path) {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((key, value)) = line.split_once('=') {
                    if std::env::var(key.trim()).is_err() {
                        std::env::set_var(key.trim(), value.trim());
                    }
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    load_env_from_file();

    let args: Vec<String> = std::env::args().collect();

    let api_key = std::env::var("OCOOLAI_API_KEY")
        .or_else(|_| args.get(1).cloned().ok_or_else(|| anyhow::anyhow!("Missing API key parameter")))
        .expect("需要提供 API Key");

    let prompt = args.get(2).cloned().unwrap_or_else(|| {
        "请写一首关于人工智能的短诗，大约4-6行。".to_string()
    });

    println!("========================================");
    println!("  ocoolAI Stream Test");
    println!("========================================");
    println!();

    let mut builder = LlmClient::from_preset("ocoolai")
        .expect("ocoolAI preset should exist")
        .with_api_key(&api_key);

    if let Ok(base_url) = std::env::var("OCOOLAI_BASE_URL") {
        builder = builder.with_base_url(&base_url);
    }

    let client = builder.build();

    // 构建流式请求
    let req = PrimitiveRequest::single_user_message(&prompt)
        .with_model("4o-mini")
        .with_stream(true);

    println!("用户: {}\n", prompt);
    println!("AI: ");

    // 发送流式请求
    let mut stream = client.stream(&req).await?;

    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                print!("{}", chunk.content);
                // 强制刷新输出
                use std::io::Write;
                std::io::stdout().flush().ok();
            }
            Err(e) => {
                eprintln!("\n错误: {}", e);
                break;
            }
        }
    }

    println!();
    println!();
    println!("========================================");
    println!("  Stream Test Complete");
    println!("========================================");

    Ok(())
}
