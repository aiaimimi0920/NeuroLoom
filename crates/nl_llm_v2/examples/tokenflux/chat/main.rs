//! TokenFlux 平台基础对话测试
//!
//! 运行方式: cargo run --example tokenflux_chat
//! 或直接运行: test.bat

use anyhow::Result;
use nl_llm_v2::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // 从环境变量或参数获取认证信息
    let api_key = std::env::var("TOKENFLUX_API_KEY")
        .unwrap_or_else(|_| args.get(1).cloned().expect("需要提供 API Key"));

    // 创建客户端
    let client = LlmClient::from_preset("tokenflux")
        .expect("Preset not found")
        .with_api_key(api_key)
        .build();

    // 构建请求
    let prompt = args.get(2).cloned().unwrap_or_else(|| "Hello!".to_string());

    let req = PrimitiveRequest::single_user_message(&prompt).with_model("gpt-3.5-turbo"); // TokenFlux supports many models, we use a standard fast one for testing

    // 发送请求
    println!("用户: {}\n", prompt);
    println!("AI:");

    let resp = client.complete(&req).await?;
    println!("{}", resp.content);

    Ok(())
}
