//! DouBaoSeed 模型列表查询
//!
//! 运行方式:
//!   方式1: cargo run -p nl_llm_v2 --example doubaoseed_models -- <api_key>
//!   方式2: 使用 test.bat（自动读取 .env.local 中的密钥）

use nl_llm_v2::{LlmClient, Capability};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("DOUBAOSEED_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: doubaoseed_models <API_KEY>");
            eprintln!("或设置 DOUBAOSEED_API_KEY 环境变量");
            std::process::exit(1);
        });

    let client = LlmClient::from_preset("doubaoseed")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    println!("========================================");
    println!("  DouBaoSeed (豆包) 可用模型列表");
    println!("========================================\n");

    // 获取模型列表
    let models = client.list_models().await?;
    for (i, model) in models.iter().enumerate() {
        println!("  {}. {} — {}", i + 1, model.id, model.description);
    }
    println!("\n共 {} 个模型", models.len());

    // ========== 演示模型别名解析 ==========
    println!("\n----------------------------------------");
    println!("模型别名速查表:");
    println!("  doubao / seed / pro  → {} (旗舰模型)", client.resolve_model("doubao"));
    println!("  code                 → {} (编码模型)", client.resolve_model("code"));
    println!("  lite                 → {} (轻量模型)", client.resolve_model("lite"));

    // ========== 演示能力对比 ==========
    println!("\n----------------------------------------");
    println!("模型能力对比:");

    let pro_model = "doubao-seed-2-0-pro-260215";
    let code_model = "doubao-seed-2-0-code-preview-latest";
    let thinking_model = "doubao-1-5-thinking-pro-250415";

    println!("\n[{}] - 旗舰模型", pro_model);
    println!("  Chat: {}", if client.has_capability(pro_model, Capability::CHAT) { "✅" } else { "❌" });
    println!("  Vision: {}", if client.has_capability(pro_model, Capability::VISION) { "✅" } else { "❌" });
    println!("  Tools: {}", if client.has_capability(pro_model, Capability::TOOLS) { "✅" } else { "❌" });
    println!("  Thinking: {}", if client.has_capability(pro_model, Capability::THINKING) { "✅" } else { "❌" });

    println!("\n[{}] - 编码模型", code_model);
    println!("  Chat: {}", if client.has_capability(code_model, Capability::CHAT) { "✅" } else { "❌" });
    println!("  Vision: {}", if client.has_capability(code_model, Capability::VISION) { "✅" } else { "❌" });
    println!("  Tools: {}", if client.has_capability(code_model, Capability::TOOLS) { "✅" } else { "❌" });

    println!("\n[{}] - 思考模型", thinking_model);
    println!("  Chat: {}", if client.has_capability(thinking_model, Capability::CHAT) { "✅" } else { "❌" });
    println!("  Thinking: {}", if client.has_capability(thinking_model, Capability::THINKING) { "✅" } else { "❌" });

    println!("\n上下文长度: {} tokens (旗舰模型)", client.max_context("doubao"));

    Ok(())
}
