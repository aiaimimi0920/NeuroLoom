//! vertex 平台测试 - auth
//!
//! 运行方式:
//!   cargo run --example vertex_auth -- path/to/sa.json
//!   cargo run --example vertex_auth -- '{"type":"service_account",...}'
//! 或直接运行: test.bat

use nl_llm_v2::{LlmClient, PrimitiveRequest};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let sa_json = load_sa_json(&args)?;

    let client = LlmClient::from_preset("vertex")
        .expect("Preset should exist")
        .with_service_account_json(&sa_json)
        .build();

    let prompt = args.get(2).cloned()
        .unwrap_or_else(|| "Hello! Please introduce yourself briefly.".to_string());

    let req = PrimitiveRequest::single_user_message(&prompt)
        .with_model("gemini-2.5-flash");

    println!("用户: {}\n", prompt);
    println!("AI:");

    let resp = client.complete(&req).await?;
    println!("{}", resp.content);

    Ok(())
}

/// 加载 SA JSON：支持环境变量、文件路径、直接 JSON 字符串
fn load_sa_json(args: &[String]) -> Result<String> {
    // 优先环境变量
    if let Ok(json) = std::env::var("GOOGLE_APPLICATION_CREDENTIALS_JSON") {
        if !json.is_empty() {
            return Ok(json);
        }
    }

    // 命令行参数
    if let Some(arg) = args.get(1) {
        if !arg.is_empty() {
            // 如果是文件路径
            let path = std::path::Path::new(arg);
            if path.exists() && path.is_file() {
                return Ok(std::fs::read_to_string(path)?);
            }
            // 尝试作为 JSON 字符串
            if arg.trim().starts_with('{') {
                return Ok(arg.clone());
            }
            // 尝试在 examples/vertex/ 目录下查找
            let fallback = std::path::Path::new("crates/nl_llm_v2/examples/vertex").join(arg);
            if fallback.exists() {
                return Ok(std::fs::read_to_string(fallback)?);
            }
        }
    }

    Err(anyhow::anyhow!("请提供 SA JSON: 文件路径或 JSON 字符串"))
}
