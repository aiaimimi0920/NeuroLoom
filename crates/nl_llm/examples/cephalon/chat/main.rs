//! cephalon 平台测试 - chat
//!
//! 运行方式: cargo run --example cephalon_chat
//! 或直接运行: test.bat
//!
//! 密钥获取: https://cephalon.cloud/apitoken/

use anyhow::Result;
use nl_llm::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // 从环境变量或命令行参数获取 API Key
    let api_key = std::env::var("CEPHALON_API_KEY")
        .ok()
        .or_else(|| args.get(1).cloned())
        .expect("需要提供 Cephalon API Key (设置环境变量 CEPHALON_API_KEY 或作为第一个参数传入)");

    // 创建客户端
    let client = LlmClient::from_preset("cephalon")
        .expect("Preset should exist")
        .with_api_key(api_key)
        .build();

    // 获取 prompt
    let prompt = args.get(2).cloned().unwrap_or_else(|| "你好！请简单介绍一下你自己。".to_string());

    // 构建请求
    let req = PrimitiveRequest::single_user_message(&prompt).with_model("gpt-4o");

    println!("用户: {}\n", prompt);
    println!("AI:");

    let resp = client.complete(&req).await?;
    println!("{}", resp.content);

    Ok(())
}
