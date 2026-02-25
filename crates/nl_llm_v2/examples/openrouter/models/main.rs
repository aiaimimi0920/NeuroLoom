//! OpenRouter 模型列表与能力检测

use nl_llm_v2::{LlmClient, model::Capability};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("OPENROUTER_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: openrouter_models <API_KEY>");
            eprintln!("或设置 OPENROUTER_API_KEY 环境变量");
            std::process::exit(1);
        });

    let client = LlmClient::from_preset("openrouter")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    println!("========================================");
    println!("  OpenRouter 模型列表 (动态 API)");
    println!("========================================\n");

    match client.list_models().await {
        Ok(models) => {
            println!("共 {} 个模型 (显示前 20 个):\n", models.len());
            for (i, m) in models.iter().take(20).enumerate() {
                println!("  {}. {} — {}", i + 1, m.id, m.description);
            }
            if models.len() > 20 {
                println!("  ... 还有 {} 个模型", models.len() - 20);
            }
        }
        Err(e) => { println!("❌ 获取失败: {}", e); std::process::exit(1); }
    }

    println!("\n----------------------------------------");
    println!("  别名解析演示");
    println!("----------------------------------------\n");

    let resolver = client.model_resolver();
    let aliases = ["gemini", "gemini-flash", "claude", "gpt4", "deepseek", "llama"];
    for alias in aliases {
        println!("  '{}' -> '{}'", alias, resolver.resolve(alias));
    }

    Ok(())
}
