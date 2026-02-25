//! Cubence 模型列表

use nl_llm_v2::LlmClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("CUBENCE_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: cubence_models <API_KEY>");
            std::process::exit(1);
        });

    let client = LlmClient::from_preset("cubence")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    println!("========================================");
    println!("  Cubence 模型列表");
    println!("========================================\n");

    match client.list_models().await {
        Ok(models) => {
            println!("共 {} 个模型:\n", models.len());
            for (i, m) in models.iter().enumerate() {
                println!("  {}. {} — {}", i + 1, m.id, m.description);
            }
        }
        Err(e) => { println!("❌ 获取失败: {}", e); }
    }

    println!("\n----------------------------------------");
    println!("  常用别名");
    println!("----------------------------------------\n");

    let aliases = [
        ("cubence / sonnet", "claude-sonnet-4-5-20250929"),
        ("4o", "gpt-4o"),
        ("4o-mini", "gpt-4o-mini"),
        ("gemini", "gemini-2.0-flash"),
        ("gemini-pro", "gemini-2.5-pro"),
    ];
    for (alias, target) in aliases {
        println!("  '{}' -> '{}'", alias, target);
    }

    Ok(())
}
