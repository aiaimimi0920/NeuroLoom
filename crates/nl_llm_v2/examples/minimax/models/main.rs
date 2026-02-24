//! MiniMax (en) 模型列表与能力检测

use nl_llm_v2::{LlmClient, model::Capability};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("MINIMAX_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| "invalid_dummy_key_for_testing".to_string());

    let client = LlmClient::from_preset("minimax")
        .expect("Preset should exist")
        .with_api_key(api_key)
        .build();

    println!("========================================");
    println!("  MiniMax 模型列表与能力检测");
    println!("========================================\n");

    // 获取模型列表
    match client.list_models().await {
        Ok(models) => {
            println!("获取成功，共有 {} 个模型:\n", models.len());
            for (i, m) in models.iter().enumerate() {
                println!("{:02}. 模型 ID: {}", i + 1, m.id);
                println!("    描述:   {}", m.description);
                println!();
            }
        }
        Err(e) => { println!("❌ 获取失败: {}", e); std::process::exit(1); }
    }

    // 能力检测
    println!("----------------------------------------");
    println!("  能力检测演示");
    println!("----------------------------------------\n");

    let resolver = client.model_resolver();
    let model = "MiniMax-M2.5";

    println!("模型: {}", model);
    println!("  上下文长度: {} tokens", resolver.max_context(model));

    let caps = [
        (Capability::CHAT, "CHAT"),
        (Capability::STREAMING, "STREAMING"),
        (Capability::TOOLS, "TOOLS"),
        (Capability::VISION, "VISION"),
    ];

    for (cap, name) in caps {
        let status = if resolver.has_capability(model, cap) { "✓" } else { "✗" };
        println!("  {} {}", status, name);
    }

    // 别名解析
    println!("\n----------------------------------------");
    println!("  别名解析演示");
    println!("----------------------------------------\n");

    let aliases = ["minimax", "m2.5", "MiniMax-M2.5"];
    for alias in aliases {
        println!("  '{}' -> '{}'", alias, resolver.resolve(alias));
    }

    Ok(())
}
