//! qiniu 平台测试 - chat
//!
//! 运行方式: cargo run --example qiniu_chat
//! 或直接运行: test.bat

use anyhow::Result;
use nl_llm_v2::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let api_key = std::env::var("QINIU_API_KEY")
        .ok()
        .or_else(|| std::env::var("NL_API_KEY").ok())
        .or_else(|| args.get(1).cloned())
        .unwrap_or_else(|| "dummy_credential".to_string());

    let prompt = args
        .get(2)
        .cloned()
        .unwrap_or_else(|| "你好！请一句话介绍 Rust 的所有权机制。".to_string());

    let client = LlmClient::build_qiniu(api_key);

    let req = PrimitiveRequest::single_user_message(&prompt).with_model("qwen-plus");

    println!("用户: {}\n", prompt);
    println!("AI:");

    let resp = client.complete(&req).await?;
    println!("{}", resp.content);

    Ok(())
}
