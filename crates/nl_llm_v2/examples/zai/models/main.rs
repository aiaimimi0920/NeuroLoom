//! Z.AI（智谱 GLM 海外版）模型列表查询
//!
//! Z.AI 是智谱 AI 的海外服务，提供 GLM 系列模型。
//! 此示例演示如何获取可用模型列表。
//!
//! ## 特性演示
//!
//! - 动态获取模型列表
//! - 静态兜底列表
//!
//! 运行方式:
//!   cargo run -p nl_llm_v2 --example zai_models -- <api_key>
//! 或设置 ZAI_API_KEY 环境变量后:
//!   cargo run -p nl_llm_v2 --example zai_models

use nl_llm_v2::LlmClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("ZAI_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: zai_models <API_KEY>");
            eprintln!("或设置 ZAI_API_KEY 环境变量");
            std::process::exit(1);
        });

    println!("========================================");
    println!("  Z.AI (智谱GLM海外版) 可用模型列表");
    println!("========================================\n");

    let client = LlmClient::from_preset("zai")
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
            // 提取前缀（如 glm-4, glm-5）
            let prefix = model.id.split('-').take(2).collect::<Vec<_>>().join("-");

            if prefix != current_prefix {
                current_prefix = prefix.clone();
                println!("\n[{}]", prefix.to_uppercase());
            }

            println!("  • {} — {}", model.id, model.description);
        }
    }

    // ========== 演示模型别名解析 ==========
    println!("\n----------------------------------------");
    println!("常用模型别名:");
    println!("  glm   → {} (旗舰模型)", client.resolve_model("glm"));
    println!("  flash → {} (快速模型)", client.resolve_model("flash"));
    println!("  vision → {} (视觉模型)", client.resolve_model("vision"));

    Ok(())
}
