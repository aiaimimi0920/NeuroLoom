//! Kimi (Moonshot) 基础对话测试
//!
//! ## 特性演示
//!
//! - Kimi 常用模型别名用法（"k2.5", "coding" 等）
//! - 可以切换不同的后端 endpoint (`api.kimi.com`)
//!
//! 运行方式:
//!   方式1: cargo run -p nl_llm_v2 --example kimi_chat -- <api_key> [prompt]
//!   方式2: 设置 KIMI_API_KEY 环境变量后直接运行
//!   方式3: 使用 test.bat（自动读取 .env.local 中的密钥）

use nl_llm_v2::{LlmClient, PrimitiveRequest, Capability};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("KIMI_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: kimi_chat <API_KEY> [prompt]");
            eprintln!("或设置 KIMI_API_KEY 环境变量");
            std::process::exit(1);
        });

    let prompt = std::env::args().nth(2)
        .unwrap_or_else(|| "写一段简单的冒泡排序，只需代码。".to_string());

    // ========== 创建客户端 ==========
    println!("========================================");
    println!("  Kimi For Coding 对话测试 + 模型别名演示");
    println!("========================================\n");

    let client = LlmClient::from_preset("kimi")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    // 如果想特定路由至 Kimi For Coding，理论上可以切换 baseUrl，不过这通常由别名或用户自己在设定时完成：
    // let client = client.with_base_url("https://api.kimi.com/v1");

    // ========== 演示模型别名 ==========
    let model_alias = "coding";  // 使用专精代码的别名
    let resolved = client.resolve_model(model_alias);
    println!("别名 '{}' 解析为: {}", model_alias, resolved);

    // 检查模型能力
    if client.has_capability(model_alias, Capability::TOOLS) {
        println!("✅ {} 支持工具调用", model_alias);
    }

    // 获取上下文窗口建议
    let (input_limit, output_limit) = client.context_window_hint(model_alias);
    println!("上下文窗口: 输入 {} / 输出 {} tokens\n", input_limit, output_limit);

    // ========== 发送请求 ==========
    println!("----------------------------------------");
    println!("用户: {}\n", prompt);

    let req = PrimitiveRequest::single_user_message(&prompt)
        .with_model(model_alias);  // 使用专门生成代码的模型别名

    let resp = client.complete(&req).await?;
    println!("AI: {}\n", resp.content);

    // 显示 token 使用情况
    if let Some(usage) = &resp.usage {
        println!("Token: prompt={}, completion={}, total={}",
            usage.prompt_tokens, usage.completion_tokens, usage.total_tokens);
    }

    Ok(())
}
