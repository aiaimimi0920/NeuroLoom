//! 通义千问 (Qwen) 基础对话测试
//!
//! ## 特性演示
//!
//! - 模型别名用法（"qwen", "coder", "max" 等）
//! - 能力检测
//! - 上下文窗口查询
//!
//! 运行方式:
//!   方式1: cargo run -p nl_llm_v2 --example qwen_chat -- <api_key> [prompt]
//!   方式2: 设置 QWEN_API_KEY 环境变量后直接运行
//!   方式3: 使用 test.bat（自动读取 .env.local 中的密钥）

use nl_llm_v2::{LlmClient, PrimitiveRequest, Capability};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("QWEN_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: qwen_chat <API_KEY> [prompt]");
            eprintln!("或设置 QWEN_API_KEY 环境变量");
            std::process::exit(1);
        });

    let prompt = std::env::args().nth(2)
        .unwrap_or_else(|| "Hello! Please introduce yourself briefly.".to_string());

    // ========== 创建客户端 ==========
    println!("========================================");
    println!("  通义千问 (Qwen) 对话测试 + 模型别名演示");
    println!("========================================\n");

    let client = LlmClient::from_preset("qwen")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    // ========== 演示模型别名 ==========
    let model_alias = "coder";  // 使用专精代码的别名
    let resolved = client.resolve_model(model_alias);
    println!("别名 '{}' 解析为: {}", model_alias, resolved);

    // 检查模型能力
    if client.has_capability(model_alias, Capability::TOOLS) {
        println!("✅ {} 支持工具调用", model_alias);
    }
    if client.has_capability(model_alias, Capability::VISION) {
        println!("✅ {} 支持多模态视觉", model_alias);
    } else {
        println!("ℹ️  {} 不支持多模态视觉（使用 'vl' 别名可测试）", model_alias);
    }

    // 获取上下文窗口建议
    let (input_limit, output_limit) = client.context_window_hint(model_alias);
    println!("上下文窗口: 输入 {} / 输出 {} tokens\n", input_limit, output_limit);

    // ========== 发送请求 ==========
    println!("----------------------------------------");
    println!("用户: {}\n", prompt);

    let req = PrimitiveRequest::single_user_message(&prompt)
        .with_model(model_alias);  // 使用别名

    let resp = client.complete(&req).await?;
    println!("AI: {}\n", resp.content);

    // 显示 token 使用情况
    if let Some(usage) = &resp.usage {
        println!("Token: prompt={}, completion={}, total={}",
            usage.prompt_tokens, usage.completion_tokens, usage.total_tokens);
    }

    Ok(())
}
