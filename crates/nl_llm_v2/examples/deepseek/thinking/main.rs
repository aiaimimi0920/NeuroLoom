//! DeepSeek 推理/思考模式测试
//!
//! DeepSeek-Reasoner 支持 Thinking 模式，提供链式思考能力。
//!
//! ## 特性演示
//!
//! - 推理模型 (deepseek-reasoner) 的使用
//! - Thinking 能力展示
//! - 与普通对话模型的对比
//!
//! 运行方式:
//!   方式1: cargo run -p nl_llm_v2 --example deepseek_thinking -- <api_key> [prompt]
//!   方式2: 使用 test.bat

use nl_llm_v2::{LlmClient, PrimitiveRequest, Capability};
use futures::StreamExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("DEEPSEEK_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: deepseek_thinking <API_KEY> [prompt]");
            eprintln!("或设置 DEEPSEEK_API_KEY 环境变量");
            std::process::exit(1);
        });

    let prompt = std::env::args().nth(2)
        .unwrap_or_else(|| "如果一个农夫有17只羊，除了9只以外都死了，农夫还有多少只羊？".to_string());

    println!("========================================");
    println!("  DeepSeek 推理模式 (Thinking) 演示");
    println!("========================================\n");

    let client = LlmClient::from_preset("deepseek")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    // ========== 模型对比 ==========
    println!("=== 模型能力对比 ===\n");

    let chat_model = "ds";  // deepseek-chat
    let think_model = "think";  // deepseek-reasoner

    println!("[{}] 对话模型", client.resolve_model(chat_model));
    println!("  Tools: {}", if client.has_capability(chat_model, Capability::TOOLS) { "✅" } else { "❌" });
    println!("  Thinking: {}", if client.has_capability(chat_model, Capability::THINKING) { "✅" } else { "❌" });

    println!("\n[{}] 推理模型", client.resolve_model(think_model));
    println!("  Tools: {}", if client.has_capability(think_model, Capability::TOOLS) { "✅" } else { "❌" });
    println!("  Thinking: {}", if client.has_capability(think_model, Capability::THINKING) { "✅" } else { "❌" });

    // ========== 使用推理模型 ==========
    println!("\n----------------------------------------");
    println!("用户: {}\n", prompt);
    println!("使用推理模型 [{}] 流式输出:", client.resolve_model(think_model));
    print!("AI: ");

    let req = PrimitiveRequest::single_user_message(&prompt)
        .with_model(think_model);  // 使用推理模型

    let mut stream = client.stream(&req).await?;
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(c) => print!("{}", c.content),
            Err(e) => {
                eprintln!("\n流式错误: {}", e);
                break;
            }
        }
    }
    println!("\n");

    // ========== 建议 ==========
    println!("----------------------------------------");
    println!("💡 使用建议:");
    println!("  - 对话模型 (ds): 适合日常对话、工具调用");
    println!("  - 推理模型 (think/r1): 适合复杂推理、数学问题、逻辑分析");

    Ok(())
}
