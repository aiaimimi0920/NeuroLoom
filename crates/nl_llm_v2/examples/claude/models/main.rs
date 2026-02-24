//! Claude API Key 模型列表示例

use nl_llm_v2::LlmClient;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .unwrap_or_else(|_| "dummy_credential".to_string());

    let client = LlmClient::from_preset("claude")
        .expect("Preset should exist")
        .with_claude_api_key(api_key)
        .build();

    println!("=== Claude 可用模型列表 ===\n");

    let models = client.list_models().await?;
    for (i, m) in models.iter().enumerate() {
        println!("  {:2}. {:<40} {}", i + 1, m.id, m.description);
    }

    println!("\n共计 {} 个模型", models.len());

    Ok(())
}
