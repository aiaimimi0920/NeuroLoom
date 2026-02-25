//! Longcat (Longcat AI) 模型列表查询测试
//!
//! ## 特性演示
//!
//! - 静态模型列表
//! - 模型别名解析
//! - 能力对比
//!
//! 运行方式:
//!   方式1: cargo run -p nl_llm_v2 --example longcat_models
//!   方式2: 使用 test.bat

use nl_llm_v2::LlmClient;
use nl_llm_v2::Capability;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("LONGCAT_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            // 静态模型列表不需要真实密钥即可查看
            "placeholder".to_string()
        });

    println!("========================================");
    println!("  Longcat 可用模型列表");
    println!("========================================\n");

    let client = LlmClient::from_preset("longcat")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    // 获取模型列表
    match client.list_models().await {
        Ok(models) => {
            println!("共 {} 个模型:\n", models.len());
            for (i, m) in models.iter().enumerate() {
                println!("  {}. {} — {}", i + 1, m.id, m.description);
            }
        }
        Err(e) => {
            println!("❌ 获取失败: {}", e);
            std::process::exit(1);
        }
    }

    // ========== 演示模型别名解析 ==========
    println!("\n----------------------------------------");
    println!("模型别名速查表:");
    println!("  longcat → {} (默认模型)", client.resolve_model("longcat"));
    println!("  flash   → {} (默认模型)", client.resolve_model("flash"));

    // ========== 演示能力配置 ==========
    println!("\n----------------------------------------");
    println!("模型能力:");

    let model = "LongCat-Flash-Chat";
    println!("\n[{}]", model);
    println!("  Chat: {}", if client.has_capability(model, Capability::CHAT) { "✅" } else { "❌" });
    println!("  Tools: {}", if client.has_capability(model, Capability::TOOLS) { "✅" } else { "❌" });
    println!("  Streaming: {}", if client.has_capability(model, Capability::STREAMING) { "✅" } else { "❌" });

    println!("\n上下文长度: {} tokens", client.max_context("flash"));

    Ok(())
}
