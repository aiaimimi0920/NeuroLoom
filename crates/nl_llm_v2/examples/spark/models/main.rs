//! 讯飞星火 模型列表查看器
//!
//! 运行方式: cargo run -p nl_llm_v2 --example spark_models

use nl_llm_v2::LlmClient;
use nl_llm_v2::model::resolver::Capability;

fn main() -> anyhow::Result<()> {
    println!("========================================");
    println!("  讯飞星火 可用模型列表");
    println!("========================================\n");

    let client = LlmClient::from_preset("spark")
        .expect("Preset should exist")
        .build();

    let models = vec![
        ("4.0Ultra", "旗舰模型"),
        ("max-32k", "长文本"),
        ("generalv3.5", "Spark Max"),
        ("pro-128k", "长上下文"),
        ("generalv3", "Spark Pro"),
        ("lite", "免费轻量"),
    ];

    for (i, (model, desc)) in models.iter().enumerate() {
        println!("  {}. {} — {}", i + 1, model, desc);
    }

    println!("\n----------------------------------------");
    println!("别名速查表:");

    let aliases = vec![
        ("spark", "4.0Ultra"),
        ("ultra", "4.0Ultra"),
        ("max", "generalv3.5"),
        ("max32k", "max-32k"),
        ("pro", "generalv3"),
        ("pro128k", "pro-128k"),
        ("lite", "lite"),
    ];

    for (alias, _) in &aliases {
        let resolved = client.resolve_model(alias);
        println!("  {:>10} → {}", alias, resolved);
    }

    println!("\n----------------------------------------");
    println!("能力对比:\n");

    for (model, _) in &models {
        let chat = if client.has_capability(model, Capability::CHAT) { "✅" } else { "❌" };
        let tools = if client.has_capability(model, Capability::TOOLS) { "✅" } else { "❌" };
        let stream = if client.has_capability(model, Capability::STREAMING) { "✅" } else { "❌" };
        let ctx = client.max_context(model);
        println!("  {:>15}: Chat {} | Tools {} | Stream {} | {}K ctx",
            model, chat, tools, stream, ctx / 1000);
    }

    Ok(())
}
