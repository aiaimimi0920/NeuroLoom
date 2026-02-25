//! AICodeMirror 模型能力检测演示
//!
//! 演示如何检测模型能力（工具调用、视觉、流式等）

use nl_llm_v2::{Capability, LlmClient};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("AICODEMIRROR_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: aicodemirror_capabilities <API_KEY>");
            std::process::exit(1);
        });

    let client = LlmClient::from_preset("aicodemirror")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    println!("========================================");
    println!("  AICodeMirror 模型能力检测");
    println!("========================================\n");

    let models = [
        ("claude-sonnet-4-6", "Claude 4.6 Sonnet"),
        ("claude-opus-4-20250514", "Claude 4 Opus"),
        ("claude-haiku-4-5-20251001", "Claude 4.5 Haiku"),
        ("claude-3-7-sonnet-20250219", "Claude 3.7 Sonnet"),
    ];

    println!("能力图例:");
    println!("  C = Chat    T = Tools    S = Streaming");
    println!("  V = Vision  K = Thinking (推理增强)\n");

    for (model_id, display_name) in models {
        let chat = client.has_capability(model_id, Capability::CHAT);
        let tools = client.has_capability(model_id, Capability::TOOLS);
        let stream = client.has_capability(model_id, Capability::STREAMING);
        let vision = client.has_capability(model_id, Capability::VISION);
        let thinking = client.has_capability(model_id, Capability::THINKING);
        let ctx = client.max_context(model_id);

        let caps = format!(
            "{}{}{}{}{}",
            if chat { "C" } else { "-" },
            if tools { "T" } else { "-" },
            if stream { "S" } else { "-" },
            if vision { "V" } else { "-" },
            if thinking { "K" } else { "-" },
        );

        println!("{} ({})", display_name, model_id);
        println!("  能力: {} | 上下文: {} tokens", caps, format_context(ctx));
    }

    println!("\n----------------------------------------");
    println!("  模型别名能力检测");
    println!("----------------------------------------\n");

    let aliases = ["sonnet", "opus", "haiku", "sonnet-4.6", "opus-4.6"];
    for alias in aliases {
        let resolved = client.resolve_model(alias);
        let vision = client.has_capability(alias, Capability::VISION);
        let thinking = client.has_capability(alias, Capability::THINKING);
        let ctx = client.max_context(alias);

        println!("'{}' -> {}", alias, resolved);
        println!("  视觉: {} | 推理: {} | 上下文: {}",
            if vision { "是" } else { "否" },
            if thinking { "是" } else { "否" },
            format_context(ctx));
    }

    Ok(())
}

fn format_context(tokens: usize) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1000 {
        format!("{:.0}K", tokens as f64 / 1000.0)
    } else {
        format!("{}", tokens)
    }
}
