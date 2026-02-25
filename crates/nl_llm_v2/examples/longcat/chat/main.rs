//! Longcat (Longcat AI) 基础对话示例
//!
//! ## 特性演示
//!
//! - 模型别名用法（"longcat", "flash"）
//! - 能力检测
//! - 上下文窗口查询
//!
//! 运行方式:
//!   方式1: cargo run -p nl_llm_v2 --example longcat_chat -- <api_key> [prompt]
//!   方式2: 设置 LONGCAT_API_KEY 环境变量后直接运行
//!   方式3: 使用 test.bat

use nl_llm_v2::{LlmClient, PrimitiveRequest, Capability};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("LONGCAT_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("需要提供 LONGCAT_API_KEY");
            std::process::exit(1);
        });

    // ========== 创建客户端 ==========
    println!("========================================");
    println!("  Longcat 对话测试 + 模型别名演示");
    println!("========================================\n");

    let client = LlmClient::from_preset("longcat")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    // ========== 演示模型别名 ==========
    let model_alias = "flash";  // 使用别名
    let resolved = client.resolve_model(model_alias);
    println!("别名 '{}' 解析为: {}", model_alias, resolved);

    // 检查模型能力
    if client.has_capability(model_alias, Capability::TOOLS) {
        println!("✅ {} 支持工具调用", model_alias);
    }
    if client.has_capability(model_alias, Capability::STREAMING) {
        println!("✅ {} 支持流式输出", model_alias);
    }

    // 获取上下文窗口建议
    let (input_limit, output_limit) = client.context_window_hint(model_alias);
    println!("上下文窗口: 输入 {} / 输出 {} tokens\n", input_limit, output_limit);

    // ========== 发送请求 ==========
    let prompt = std::env::args().nth(2)
        .unwrap_or_else(|| "用一句话介绍一下你自己。".to_string());

    println!("----------------------------------------");
    println!("用户: {}\n", prompt);

    let req = PrimitiveRequest::single_user_message(&prompt)
        .with_model(model_alias);  // 使用别名

    match client.complete(&req).await {
        Ok(resp) => {
            println!("AI: {}", resp.content);

            if let Some(usage) = resp.usage {
                println!(
                    "\n[Token 用量: prompt={}, completion={}, total={}]",
                    usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
                );
            }
        }
        Err(e) => {
            eprintln!("请求失败: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
