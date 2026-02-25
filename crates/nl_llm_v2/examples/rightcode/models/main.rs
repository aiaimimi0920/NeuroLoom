//! RightCode 模型列表

use nl_llm_v2::LlmClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("RIGHTCODE_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| { eprintln!("用法: rightcode_models <API_KEY>"); std::process::exit(1); });

    let client = LlmClient::from_preset("rightcode")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    println!("========================================");
    println!("  RightCode 模型列表");
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
        ("rightcode / codex-mini", "gpt-5.1-codex-mini"),
        ("codex", "gpt-5.1-codex"),
        ("codex-max", "gpt-5.1-codex-max"),
        ("5", "gpt-5"), ("5.1", "gpt-5.1"),
        ("5.2", "gpt-5.2"), ("5.3", "gpt-5.3-codex"),
    ];
    for (a, t) in aliases { println!("  '{}' -> '{}'", a, t); }
    Ok(())
}
