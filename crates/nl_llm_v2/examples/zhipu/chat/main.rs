//! 智谱 BigModel (GLM国内版) 基础对话测试
//!
//! 运行方式:
//!   方式1: cargo run -p nl_llm_v2 --example zhipu_chat -- <api_key> [prompt]
//!   方式2: 使用 test.bat（自动读取 .env.local 中的密钥）

use nl_llm_v2::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("ZHIPU_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: zhipu_chat <API_KEY> [prompt]");
            eprintln!("或设置 ZHIPU_API_KEY 环境变量");
            std::process::exit(1);
        });

    let prompt = std::env::args().nth(2)
        .unwrap_or_else(|| "Hello! Please introduce yourself briefly.".to_string());

    let client = LlmClient::from_preset("zhipu")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    let req = PrimitiveRequest::single_user_message(&prompt)
        .with_model("glm-5");

    println!("用户: {}\n", prompt);
    let resp = client.complete(&req).await?;
    println!("AI: {}\n", resp.content);
    if let Some(usage) = &resp.usage {
        println!("Token: prompt={}, completion={}, total={}",
            usage.prompt_tokens, usage.completion_tokens, usage.total_tokens);
    }

    Ok(())
}
