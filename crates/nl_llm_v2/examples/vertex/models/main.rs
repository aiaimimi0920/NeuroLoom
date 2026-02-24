//! vertex 平台测试 - models
//!
//! 通过 Vertex AI 列出可用模型。
//!
//! 运行方式:
//!   cargo run --example vertex_models -- path/to/sa.json
//! 或直接运行: test.bat

use nl_llm_v2::LlmClient;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let sa_json = load_sa_json(&args)?;

    let client = LlmClient::from_preset("vertex")
        .expect("Preset should exist")
        .with_service_account_json(&sa_json)
        .build();

    println!("=== Vertex AI 可用模型列表 ===\n");

    let models = client.list_models().await?;

    for (index, m) in models.iter().enumerate() {
        if !m.description.is_empty() {
            println!("  {}. {:<40} {}", index + 1, m.id, m.description);
        } else {
            println!("  {}. {}", index + 1, m.id);
        }
    }

    println!("\n共计 {} 个模型", models.len());

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
