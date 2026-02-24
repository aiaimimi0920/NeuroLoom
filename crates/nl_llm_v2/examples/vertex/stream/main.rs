//! vertex 平台测试 - stream
//!
//! 运行方式:
//!   cargo run --example vertex_stream -- path/to/sa.json [prompt]
//! 或直接运行: test.bat

use nl_llm_v2::{LlmClient, PrimitiveRequest};
use anyhow::Result;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let sa_json = load_sa_json(&args)?;

    let client = LlmClient::from_preset("vertex")
        .expect("Preset should exist")
        .with_service_account_json(&sa_json)
        .build();

    let prompt = args.get(2).cloned()
        .unwrap_or_else(|| "Hello! Tell me a short story.".to_string());

    let req = PrimitiveRequest::single_user_message(&prompt)
        .with_model("gemini-2.5-flash");

    println!("用户: {}\n", prompt);
    println!("AI (Stream):");

    let mut stream = client.stream(&req).await?;

    use std::io::Write;
    while let Some(chunk_res) = stream.next().await {
        match chunk_res {
            Ok(chunk) => {
                print!("{}", chunk.content);
                std::io::stdout().flush().unwrap();
            }
            Err(e) => {
                eprintln!("\n流读取中断: {}", e);
                break;
            }
        }
    }
    println!();

    Ok(())
}

/// 加载 SA JSON：支持环境变量、文件路径、直接 JSON 字符串
fn load_sa_json(args: &[String]) -> Result<String> {
    if let Ok(json) = std::env::var("GOOGLE_APPLICATION_CREDENTIALS_JSON") {
        if !json.is_empty() { return Ok(json); }
    }
    if let Some(arg) = args.get(1) {
        if !arg.is_empty() {
            let path = std::path::Path::new(arg);
            if path.exists() && path.is_file() {
                return Ok(std::fs::read_to_string(path)?);
            }
            if arg.trim().starts_with('{') {
                return Ok(arg.clone());
            }
            let fallback = std::path::Path::new("crates/nl_llm_v2/examples/vertex").join(arg);
            if fallback.exists() { return Ok(std::fs::read_to_string(fallback)?); }
        }
    }
    Err(anyhow::anyhow!("请提供 SA JSON: 文件路径或 JSON 字符串"))
}
