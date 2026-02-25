//! Cohere 模型列表

use nl_llm_v2::LlmClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("========================================");
    println!("  Cohere 模型列表");
    println!("========================================\n");

    let client = LlmClient::from_preset("cohere")
        .expect("Preset should exist")
        .with_api_key("placeholder")
        .build();

    match client.list_models().await {
        Ok(models) => {
            println!("共 {} 个模型:\n", models.len());
            for (i, m) in models.iter().enumerate() {
                println!("  {}. {} — {}", i + 1, m.id, m.description);
            }
        }
        Err(e) => { println!("获取失败: {}", e); }
    }

    println!("\n----------------------------------------");
    println!("  常用别名");
    println!("----------------------------------------\n");
    let aliases = [
        ("cohere / command / command-a", "command-a-03-2025"),
        ("vision", "command-a-vision-07-2025"),
        ("reasoning", "command-a-reasoning-08-2025"),
        ("translate", "command-a-translate-08-2025"),
        ("r+", "command-r-plus-08-2024"),
        ("r", "command-r-08-2024"),
        ("r7b", "command-r7b-12-2024"),
    ];
    for (a, t) in aliases { println!("  '{}' -> '{}'", a, t); }

    println!("\n----------------------------------------");
    println!("  认证配置说明");
    println!("----------------------------------------\n");
    println!("  环境变量: COHERE_API_KEY=xxx");
    println!("\n  密钥类型:");
    println!("    - 生产密钥: 付费使用，无速率限制");
    println!("    - 试用密钥: 免费，20 RPM 限制");
    println!("\n  获取密钥: https://dashboard.cohere.com/api-keys");

    Ok(())
}
