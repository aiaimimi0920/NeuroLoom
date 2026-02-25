//! AICodeMirror 模型列表与别名演示
//!
//! 演示如何获取模型列表和使用模型别名

use nl_llm_v2::LlmClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("AICODEMIRROR_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: aicodemirror_models <API_KEY>");
            std::process::exit(1);
        });

    let client = LlmClient::from_preset("aicodemirror")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    println!("========================================");
    println!("  AICodeMirror 模型列表");
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
        ("aicodemirror / sonnet / claude", "claude-sonnet-4-5-20250929", "默认模型"),
        ("sonnet-4.6", "claude-sonnet-4-6", "最新 4.6 版本"),
        ("opus", "claude-opus-4-20250514", "旗舰模型"),
        ("opus-4.6", "claude-opus-4-6", "最新旗舰"),
        ("haiku / haiku-4.5", "claude-haiku-4-5-20251001", "快速模型"),
        ("3.7", "claude-3-7-sonnet-20250219", "扩展思考"),
        ("3.5", "claude-3-5-sonnet-20241022", "经典版本"),
    ];

    for (alias, target, desc) in aliases {
        println!("  '{}' -> '{}'", alias, target);
        println!("      {}", desc);
    }

    println!("\n----------------------------------------");
    println!("  别名解析测试");
    println!("----------------------------------------\n");

    for alias in ["sonnet", "opus", "haiku", "sonnet-4.6", "opus-4.6"] {
        let resolved = client.resolve_model(alias);
        println!("  resolve('{}') = '{}'", alias, resolved);
    }

    Ok(())
}
