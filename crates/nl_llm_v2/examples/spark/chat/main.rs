//! 讯飞星火 基础对话
//!
//! 运行方式: cargo run -p nl_llm_v2 --example spark_chat -- <api_key:api_secret> [prompt]

use nl_llm_v2::{LlmClient, PrimitiveRequest, Capability};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("SPARK_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("需要提供 SPARK_API_KEY (格式: APIKey:APISecret)");
            std::process::exit(1);
        });

    let prompt = std::env::args().nth(2)
        .unwrap_or_else(|| "用一句话介绍一下讯飞星火大模型。".to_string());

    println!("========================================");
    println!("  讯飞星火 对话测试 + 模型别名演示");
    println!("========================================\n");

    let client = LlmClient::from_preset("spark_x")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    // 演示别名
    let model_alias = "ultra";
    let resolved = client.resolve_model(model_alias);
    println!("别名 '{}' 解析为: {}", model_alias, resolved);

    if client.has_capability(model_alias, Capability::TOOLS) {
        println!("✅ {} 支持工具调用", model_alias);
    }
    if client.has_capability(model_alias, Capability::STREAMING) {
        println!("✅ {} 支持流式输出", model_alias);
    }

    let (input_limit, output_limit) = client.context_window_hint(model_alias);
    println!("上下文窗口: 输入 {} / 输出 {} tokens\n", input_limit, output_limit);

    println!("----------------------------------------");
    println!("用户: {}\n", prompt);

    let req = PrimitiveRequest::single_user_message(&prompt);

    match client.complete(&req).await {
        Ok(resp) => {
            println!("AI: {}", resp.content);
            if let Some(usage) = resp.usage {
                println!("\n[Token 用量: prompt={}, completion={}, total={}]",
                    usage.prompt_tokens, usage.completion_tokens, usage.total_tokens);
            }
        }
        Err(e) => {
            eprintln!("请求失败: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
