//! AiHubMix 模型列表与能力检测

use nl_llm_v2::{LlmClient, model::Capability};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("AIHUBMIX_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: aihubmix_models <API_KEY>");
            eprintln!("或设置 AIHUBMIX_API_KEY 环境变量");
            std::process::exit(1);
        });

    let client = LlmClient::from_preset("aihubmix")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    println!("========================================");
    println!("  AiHubMix 模型列表与能力检测");
    println!("========================================\n");

    match client.list_models().await {
        Ok(models) => {
            println!("共 {} 个模型:\n", models.len());
            for (i, m) in models.iter().enumerate() {
                println!("  {}. {} — {}", i + 1, m.id, m.description);
            }
        }
        Err(e) => { println!("❌ 获取失败: {}", e); std::process::exit(1); }
    }

    println!("\n----------------------------------------");
    println!("  能力检测演示");
    println!("----------------------------------------\n");

    let resolver = client.model_resolver();

    let test_models = ["gpt-4o-free", "gpt-4.1-free", "gemini-2.0-flash-free", "claude-sonnet-4-5-20250929"];
    for model in test_models {
        println!("模型: {} ({}K context)", model, resolver.max_context(model) / 1000);
        let caps = [
            (Capability::CHAT, "CHAT"),
            (Capability::STREAMING, "STREAM"),
            (Capability::TOOLS, "TOOLS"),
            (Capability::THINKING, "THINK"),
            (Capability::VISION, "VISION"),
        ];
        let active: Vec<&str> = caps.iter()
            .filter(|(cap, _)| resolver.has_capability(model, *cap))
            .map(|(_, name)| *name)
            .collect();
        println!("  能力: {}\n", active.join(" | "));
    }

    println!("----------------------------------------");
    println!("  别名解析演示");
    println!("----------------------------------------\n");

    let aliases = ["aihubmix", "4o", "4.1", "gemini", "sonnet", "opus"];
    for alias in aliases {
        println!("  '{}' -> '{}'", alias, resolver.resolve(alias));
    }

    Ok(())
}
