//! Qwen OAuth 对话测试
//!
//! 使用 Qwen OAuth 认证进行对话。

use nl_llm_v2::{LlmClient, PrimitiveRequest};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let cache_path = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("qwen")
        .join("token.json");

    let client = LlmClient::from_preset("qwen")
        .expect("Preset should exist")
        .with_qwen_oauth(&cache_path)
        .build();

    let prompt = args.get(1).cloned()
        .unwrap_or_else(|| "Hello! Please introduce yourself briefly.".to_string());

    let req = PrimitiveRequest::single_user_message(&prompt)
        .with_model("qwen3-coder-plus");

    println!("用户: {}\n", prompt);
    println!("AI:");

    let resp = client.complete(&req).await?;
    println!("{}", resp.content);

    Ok(())
}
