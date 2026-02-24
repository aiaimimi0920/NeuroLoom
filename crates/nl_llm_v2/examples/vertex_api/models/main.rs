//! vertex_api 平台测试 - models
//!
//! 通过 Vertex AI (API Key 模式) 列出可用模型。
//!
//! 运行方式:
//!   cargo run --example vertex_api_models -- <api_key>
//! 或直接运行: test.bat

use nl_llm_v2::LlmClient;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let api_key = std::env::var("VERTEX_API_KEY").ok()
        .or_else(|| args.get(1).cloned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            eprintln!("Error: 请提供 Vertex API Key");
            std::process::exit(1);
        });

    let client = LlmClient::from_preset("vertex_api")
        .expect("Preset should exist")
        .with_vertex_api_key(&api_key)
        .build();

    println!("=== Vertex AI (API Key) 可用模型列表 ===\n");

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
