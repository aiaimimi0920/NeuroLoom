//! ModelScope (魔搭) 模型列表与能力检测

use nl_llm_v2::{LlmClient, model::Capability};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("MODELSCOPE_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: modelscope_models <ACCESS_TOKEN>");
            eprintln!("或设置 MODELSCOPE_API_KEY 环境变量");
            std::process::exit(1);
        });

    let client = LlmClient::from_preset("modelscope")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    println!("========================================");
    println!("  ModelScope (魔搭) 模型列表与能力检测");
    println!("========================================\n");

    // 获取模型列表
    match client.list_models().await {
        Ok(models) => {
            println!("共 {} 个模型:\n", models.len());
            for (i, m) in models.iter().enumerate() {
                println!("  {}. {} — {}", i + 1, m.id, m.description);
            }
        }
        Err(e) => { println!("❌ 获取失败: {}", e); std::process::exit(1); }
    }

    // 能力检测
    println!("\n----------------------------------------");
    println!("  能力检测演示");
    println!("----------------------------------------\n");

    let resolver = client.model_resolver();
    let model = "Qwen/Qwen3-235B-A22B";

    println!("模型: {}", model);
    println!("  上下文长度: {} tokens", resolver.max_context(model));

    let caps = [
        (Capability::CHAT, "CHAT"),
        (Capability::STREAMING, "STREAMING"),
        (Capability::TOOLS, "TOOLS"),
        (Capability::THINKING, "THINKING"),
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

    let aliases = ["modelscope", "qwen3", "coder", "qvq"];
    for alias in aliases {
        println!("  '{}' -> '{}'", alias, resolver.resolve(alias));
    }

    Ok(())
}
