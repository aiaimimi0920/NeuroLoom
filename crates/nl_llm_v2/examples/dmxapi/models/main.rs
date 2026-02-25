//! DMXAPI 模型列表与能力检测

use nl_llm_v2::{LlmClient, model::Capability};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("DMXAPI_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: dmxapi_models <API_KEY>");
            eprintln!("或设置 DMXAPI_API_KEY 环境变量");
            std::process::exit(1);
        });

    let client = LlmClient::from_preset("dmxapi")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    println!("========================================");
    println!("  DMXAPI 模型列表与能力检测");
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
    println!("  别名解析演示");
    println!("----------------------------------------\n");

    let resolver = client.model_resolver();
    let aliases = ["dmxapi", "sonnet", "opus", "4o", "4.1"];
    for alias in aliases {
        println!("  '{}' -> '{}'", alias, resolver.resolve(alias));
    }

    Ok(())
}
