//! KAT-Coder (StreamLake) 模型列表查询
//!
//! ## 特性演示
//!
//! - 静态模型列表
//! - 模型别名解析
//! - 能力对比
//!
//! 运行方式:
//!   方式1: cargo run -p nl_llm_v2 --example kat_coder_models
//!   方式2: 使用 test.bat

use nl_llm_v2::LlmClient;
use nl_llm_v2::Capability;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("KAT_CODER_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            // 静态模型列表不需要真实密钥即可查看
            "placeholder".to_string()
        });

    println!("========================================");
    println!("  KAT-Coder 可用模型列表");
    println!("========================================\n");

    let client = LlmClient::from_preset("kat_coder")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    // 获取模型列表
    match client.list_models().await {
        Ok(models) => {
            println!("共 {} 个可用模型:\n", models.len());
            for (i, m) in models.iter().enumerate() {
                println!("  {}. {} — {}", i + 1, m.id, m.description);
            }
        }
        Err(e) => {
            eprintln!("获取模型列表失败: {}", e);
            std::process::exit(1);
        }
    }

    // ========== 演示模型别名解析 ==========
    println!("\n----------------------------------------");
    println!("模型别名速查表:");
    println!("  pro      → {} (旗舰模型)", client.resolve_model("pro"));
    println!("  pro-v1   → {} (旗舰 V1)", client.resolve_model("pro-v1"));
    println!("  air      → {} (轻量模型)", client.resolve_model("air"));

    // ========== 演示能力对比 ==========
    println!("\n----------------------------------------");
    println!("模型能力对比:");

    let pro_model = "kat-coder-pro";
    let air_model = "kat-coder-air-v1";

    println!("\n[{}] - 旗舰模型", pro_model);
    println!("  Chat: {}", if client.has_capability(pro_model, Capability::CHAT) { "✅" } else { "❌" });
    println!("  Tools: {}", if client.has_capability(pro_model, Capability::TOOLS) { "✅" } else { "❌" });
    println!("  Streaming: {}", if client.has_capability(pro_model, Capability::STREAMING) { "✅" } else { "❌" });

    println!("\n[{}] - 轻量模型", air_model);
    println!("  Chat: {}", if client.has_capability(air_model, Capability::CHAT) { "✅" } else { "❌" });
    println!("  Tools: {}", if client.has_capability(air_model, Capability::TOOLS) { "✅" } else { "❌" });
    println!("  Streaming: {}", if client.has_capability(air_model, Capability::STREAMING) { "✅" } else { "❌" });

    println!("\n上下文长度: {} tokens (所有模型)", client.max_context("pro"));

    Ok(())
}
