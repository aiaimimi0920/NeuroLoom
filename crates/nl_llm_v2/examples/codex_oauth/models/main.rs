//! Codex OAuth 可用模型列表示例

use nl_llm_v2::LlmClient;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let cache_path = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("codex")
        .join("token.json");

    let client = LlmClient::from_preset("codex_oauth")
        .expect("Preset should exist")
        .with_codex_oauth(&cache_path)
        .build();

    println!("=== Codex 可用模型列表 ===\n");

    let models = client.list_models().await?;
    for (i, m) in models.iter().enumerate() {
        println!("  {:2}. {:<30} {}", i + 1, m.id, m.description);
    }

    println!("\n共计 {} 个模型", models.len());

    Ok(())
}
