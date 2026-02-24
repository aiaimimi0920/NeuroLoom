//! Kimi 可用模型列表

use nl_llm_v2::LlmClient;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let cache_path = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("kimi")
        .join("token.json");

    let client = LlmClient::from_preset("kimi")
        .expect("Preset should exist")
        .with_kimi_oauth(&cache_path)
        .build();

    println!("=== Kimi 可用模型列表 ===\n");

    match client.list_models().await {
        Ok(models) => {
            for (i, m) in models.iter().enumerate() {
                println!("  {}. {:<30} {}", i + 1, m.id, m.description);
            }
            println!("\n共计 {} 个模型", models.len());
        }
        Err(e) => {
            eprintln!("获取模型列表失败: {}", e);
            println!("\n预配置模型:");
            println!("  - kimi-k2");
            println!("  - kimi-k2-thinking");
            println!("  - kimi-k2.5");
        }
    }

    Ok(())
}
