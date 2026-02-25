//! Kimi (Moonshot) 模型列表查询
//!
//! ## 特性演示
//!
//! - Kimi/Moonshot 静态精选模型列表
//! - 模型别名解析速查表
//!
//! 运行方式: cargo run -p nl_llm_v2 --example kimi_models

use nl_llm_v2::LlmClient;
use nl_llm_v2::Capability;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("KIMI_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            "placeholder".to_string()
        });

    println!("========================================");
    println!("  Kimi (Moonshot) 可用模型列表");
    println!("========================================\n");

    let client = LlmClient::from_preset("kimi")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    // 获取模型列表
    let models = client.list_models().await?;
    for (i, model) in models.iter().enumerate() {
        println!("  {}. {} — {}", i + 1, model.id, model.description);
    }
    println!("\n共 {} 个模型", models.len());

    // ========== 演示模型别名解析 ==========
    println!("\n----------------------------------------");
    println!("模型别名速查表:");
    println!("  kimi / k2.5   → {} (最新旗舰大模型)", client.resolve_model("k2.5"));
    println!("  moonshot-32k  → {} (前代稳定版)", client.resolve_model("moonshot-32k"));
    println!("  coding        → {} (专门优化代码生成的版本)", client.resolve_model("coding"));

    // ========== 演示能力对比 ==========
    println!("\n----------------------------------------");
    println!("能力对比:");

    let coding_model = "kimi-for-coding";
    let base_model = "kimi-k2.5";

    println!("\n[{}]", coding_model);
    println!("  Chat: {}", if client.has_capability(coding_model, Capability::CHAT) { "✅" } else { "❌" });
    println!("  Tools: {}", if client.has_capability(coding_model, Capability::TOOLS) { "✅" } else { "❌" });

    println!("\n[{}]", base_model);
    println!("  Chat: {}", if client.has_capability(base_model, Capability::CHAT) { "✅" } else { "❌" });
    println!("  Tools: {}", if client.has_capability(base_model, Capability::TOOLS) { "✅" } else { "❌" });

    println!("\n上下文长度: {} tokens (kimi-k2.5)", client.max_context("k2.5"));

    Ok(())
}
