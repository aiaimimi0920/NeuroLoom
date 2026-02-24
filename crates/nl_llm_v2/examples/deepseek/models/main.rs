//! DeepSeek 模型列表查询
//!
//! ## 特性演示
//!
//! - 静态模型列表
//! - 模型别名解析
//!
//! 运行方式: cargo run -p nl_llm_v2 --example deepseek_models

use nl_llm_v2::LlmClient;
use nl_llm_v2::Capability;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("DEEPSEEK_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            // DeepSeek 的静态模型列表不需要真实密钥即可查看
            "placeholder".to_string()
        });

    println!("========================================");
    println!("  DeepSeek 可用模型列表");
    println!("========================================\n");

    let client = LlmClient::from_preset("deepseek")
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
    println!("  ds      → {} (对话模型)", client.resolve_model("ds"));
    println!("  chat    → {} (对话模型)", client.resolve_model("chat"));
    println!("  r1      → {} (推理模型)", client.resolve_model("r1"));
    println!("  think   → {} (推理模型)", client.resolve_model("think"));
    println!("  reasoner → {} (推理模型)", client.resolve_model("reasoner"));

    // ========== 演示能力对比 ==========
    println!("\n----------------------------------------");
    println!("模型能力对比:");

    let chat_model = "deepseek-chat";
    let reasoner_model = "deepseek-reasoner";

    println!("\n[{}]", chat_model);
    println!("  Chat: {}", if client.has_capability(chat_model, Capability::CHAT) { "✅" } else { "❌" });
    println!("  Tools: {}", if client.has_capability(chat_model, Capability::TOOLS) { "✅" } else { "❌" });
    println!("  Streaming: {}", if client.has_capability(chat_model, Capability::STREAMING) { "✅" } else { "❌" });
    println!("  Thinking: {}", if client.has_capability(chat_model, Capability::THINKING) { "✅" } else { "❌" });

    println!("\n[{}]", reasoner_model);
    println!("  Chat: {}", if client.has_capability(reasoner_model, Capability::CHAT) { "✅" } else { "❌" });
    println!("  Tools: {}", if client.has_capability(reasoner_model, Capability::TOOLS) { "✅" } else { "❌" });
    println!("  Streaming: {}", if client.has_capability(reasoner_model, Capability::STREAMING) { "✅" } else { "❌" });
    println!("  Thinking: {}", if client.has_capability(reasoner_model, Capability::THINKING) { "✅" } else { "❌" });

    println!("\n上下文长度: {} tokens (所有模型)", client.resolve_model("ds"));

    Ok(())
}
