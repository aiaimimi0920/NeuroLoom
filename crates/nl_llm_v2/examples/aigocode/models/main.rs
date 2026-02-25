//! AIGoCode 模型列表与别名演示
//!
//! 演示如何获取模型列表和使用模型别名

use nl_llm_v2::LlmClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("AIGOCODE_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: aigocode_models <API_KEY>");
            std::process::exit(1);
        });

    let client = LlmClient::from_preset("aigocode")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    println!("========================================");
    println!("  AIGoCode 模型列表");
    println!("========================================\n");

    match client.list_models().await {
        Ok(models) => {
            println!("共 {} 个模型:\n", models.len());
            for (i, m) in models.iter().enumerate() {
                println!("  {}. {} — {}", i + 1, m.id, m.description);
            }
        }
        Err(e) => {
            println!("获取失败: {}", e);
        }
    }

    println!("\n----------------------------------------");
    println!("  模型别名对照表");
    println!("----------------------------------------\n");

    let aliases = [
        ("aigocode / sonnet / claude", "claude-sonnet-4-5-20250929", "默认模型，Claude Sonnet 4.5"),
        ("4o", "gpt-4o", "GPT-4o 多模态"),
        ("4o-mini", "gpt-4o-mini", "GPT-4o Mini 轻量版"),
        ("gemini", "gemini-2.0-flash", "Gemini 2.0 Flash"),
        ("deepseek", "deepseek-chat", "DeepSeek V3"),
        ("r1", "deepseek-reasoner", "DeepSeek R1 推理模型"),
    ];

    for (alias, target, desc) in aliases {
        println!("  '{}' -> '{}'", alias, target);
        println!("      {}", desc);
    }

    println!("\n----------------------------------------");
    println!("  别名解析测试");
    println!("----------------------------------------\n");

    for alias in ["sonnet", "4o", "gemini", "r1"] {
        let resolved = client.resolve_model(alias);
        println!("  resolve('{}') = '{}'", alias, resolved);
    }

    Ok(())
}
