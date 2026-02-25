//! PackyCode 模型列表与别名

use nl_llm_v2::LlmClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("PACKYCODE_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: packycode_models <API_KEY>");
            eprintln!("或设置 PACKYCODE_API_KEY 环境变量");
            std::process::exit(1);
        });

    let client = LlmClient::from_preset("packycode")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    println!("========================================");
    println!("  PackyCode 模型列表");
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
    println!("  常用别名");
    println!("----------------------------------------\n");

    let aliases = [
        ("packycode / 4o-mini", "gpt-4o-mini"),
        ("4o", "gpt-4o"),
        ("4.1", "gpt-4.1"),
        ("sonnet / claude", "claude-sonnet-4-5-20250929"),
        ("gemini", "gemini-2.0-flash"),
        ("deepseek", "deepseek-chat"),
        ("r1", "deepseek-reasoner"),
    ];
    for (alias, target) in aliases {
        println!("  '{}' -> '{}'", alias, target);
    }

    Ok(())
}
