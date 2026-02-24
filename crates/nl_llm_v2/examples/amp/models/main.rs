//! Sourcegraph Amp 模型列表查询
//!
//! Amp (ampcode.com) 聚合了多个后端供应商的模型。
//! 此示例演示如何获取可用模型列表。
//!
//! ## 特性演示
//!
//! - 动态获取模型列表
//! - 静态兜底列表
//!
//! 运行方式:
//!   cargo run -p nl_llm_v2 --example amp_models -- <api_key>
//! 或设置 AMP_API_KEY 环境变量后:
//!   cargo run -p nl_llm_v2 --example amp_models

use nl_llm_v2::LlmClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("AMP_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: amp_models <API_KEY>");
            eprintln!("或设置 AMP_API_KEY 环境变量");
            std::process::exit(1);
        });

    println!("========================================");
    println!("  Amp CLI 可用模型列表");
    println!("========================================\n");

    let client = LlmClient::from_preset("amp")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    // 获取模型列表（API 优先，静态兜底）
    let models = client.list_models().await?;

    if models.is_empty() {
        println!("⚠️  未获取到模型列表");
    } else {
        // 按模型 ID 排序显示
        let mut models = models;
        models.sort_by(|a, b| a.id.cmp(&b.id));

        println!("共 {} 个可用模型:\n", models.len());

        // 分组显示
        let mut current_prefix = String::new();
        for model in &models {
            // 提取前缀（如 gpt, claude, gemini）
            let prefix = model.id.split('-').next().unwrap_or("other");

            if prefix != current_prefix {
                current_prefix = prefix.to_string();
                println!("\n[{}]", prefix.to_uppercase());
            }

            println!("  • {} — {}", model.id, model.description);
        }
    }

    // ========== 演示模型别名解析 ==========
    println!("\n----------------------------------------");
    println!("常用模型别名:");
    println!("  best  → {} (最强能力)", client.resolve_model("best"));
    println!("  fast  → {} (快速响应)", client.resolve_model("fast"));
    println!("  cheap → {} (低成本)", client.resolve_model("cheap"));
    println!("  claude → {} (Claude Sonnet)", client.resolve_model("claude"));

    Ok(())
}
