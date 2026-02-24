//! Qwen 模型列表测试
//!
//! 注意：Qwen OAuth 模式下暂无标准的 list_models API，
//! 此处显示预配置的模型列表。

use nl_llm_v2::LlmClient;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let cache_path = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("qwen")
        .join("token.json");

    let client = LlmClient::from_preset("qwen")
        .expect("Preset should exist")
        .with_qwen_oauth(&cache_path)
        .build();

    println!("=== Qwen 可用模型列表 ===\n");

    match client.list_models().await {
        Ok(models) => {
            for (index, m) in models.iter().enumerate() {
                if !m.description.is_empty() {
                    println!("  {}. {:<30} {}", index + 1, m.id, m.description);
                } else {
                    println!("  {}. {}", index + 1, m.id);
                }
            }
            println!("\n共计 {} 个模型", models.len());
        }
        Err(e) => {
            eprintln!("获取模型列表失败: {}", e);
            println!("\n预配置模型:");
            println!("  - qwen3-coder-plus");
            println!("  - qwen3-coder-flash");
            println!("  - coder-model");
            println!("  - vision-model");
        }
    }

    Ok(())
}
