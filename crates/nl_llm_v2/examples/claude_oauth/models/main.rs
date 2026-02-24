//! Claude OAuth 可用模型列表示例

use nl_llm_v2::LlmClient;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let cache_path = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("anthropic")
        .join("token.json");

    let client = LlmClient::from_preset("claude_oauth")
        .expect("Preset should exist")
        .with_claude_oauth(&cache_path)
        .build();

    println!("=== Claude 可用模型列表 ===\n");

    let models = client.list_models().await?;
    for (i, m) in models.iter().enumerate() {
        println!("  {:2}. {:<40} {}", i + 1, m.id, m.description);
    }

    println!("\n共计 {} 个模型", models.len());

    Ok(())
}
