//! 通义千问 (Qwen) 模型列表查询
//!
//! ## 特性演示
//!
//! - 静态模型列表
//! - 模型别名解析快查表
//!
//! 运行方式: cargo run -p nl_llm_v2 --example qwen_models

use nl_llm_v2::LlmClient;
use nl_llm_v2::Capability;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("QWEN_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            // Qwen 的静态模型列表不需要真实密钥即可查看
            "placeholder".to_string()
        });

    println!("========================================");
    println!("  通义千问 (Qwen) 可用模型列表");
    println!("========================================\n");

    let client = LlmClient::from_preset("qwen")
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
    println!("  qwen / plus   → {} (默认均衡模型)", client.resolve_model("plus"));
    println!("  max           → {} (旗舰模型)", client.resolve_model("max"));
    println!("  turbo         → {} (极速模型)", client.resolve_model("turbo"));
    println!("  coder         → {} (开源代码专精)", client.resolve_model("coder"));
    println!("  vl            → {} (多模态视觉)", client.resolve_model("vl"));
    println!("  qwq           → {} (推理思考)", client.resolve_model("qwq"));

    // ========== 演示能力对比 ==========
    println!("\n----------------------------------------");
    println!("模型能力对比:");

    let coder_model = "qwen2.5-coder-32b-instruct";
    let vl_model = "qwen-vl-max";
    let qwq_model = "qwq-plus";

    println!("\n[{}] - 代码模型", coder_model);
    println!("  Chat: {}", if client.has_capability(coder_model, Capability::CHAT) { "✅" } else { "❌" });
    println!("  Tools: {}", if client.has_capability(coder_model, Capability::TOOLS) { "✅" } else { "❌" });
    println!("  Vision: {}", if client.has_capability(coder_model, Capability::VISION) { "✅" } else { "❌" });
    println!("  Thinking: {}", if client.has_capability(coder_model, Capability::THINKING) { "✅" } else { "❌" });

    println!("\n[{}] - 视觉模型", vl_model);
    println!("  Chat: {}", if client.has_capability(vl_model, Capability::CHAT) { "✅" } else { "❌" });
    println!("  Tools: {}", if client.has_capability(vl_model, Capability::TOOLS) { "✅" } else { "❌" });
    println!("  Vision: {}", if client.has_capability(vl_model, Capability::VISION) { "✅" } else { "❌" });
    println!("  Thinking: {}", if client.has_capability(vl_model, Capability::THINKING) { "✅" } else { "❌" });

    println!("\n[{}] - 推理模型", qwq_model);
    println!("  Chat: {}", if client.has_capability(qwq_model, Capability::CHAT) { "✅" } else { "❌" });
    println!("  Tools: {}", if client.has_capability(qwq_model, Capability::TOOLS) { "✅" } else { "❌" });
    println!("  Vision: {}", if client.has_capability(qwq_model, Capability::VISION) { "✅" } else { "❌" });
    println!("  Thinking: {}", if client.has_capability(qwq_model, Capability::THINKING) { "✅" } else { "❌" });

    println!("\n上下文长度: {} tokens (Qwen-Plus)", client.max_context("qwen-plus"));

    Ok(())
}
