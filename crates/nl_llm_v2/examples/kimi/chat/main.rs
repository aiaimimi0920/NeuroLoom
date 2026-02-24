//! Kimi OAuth 对话测试

use nl_llm_v2::{LlmClient, PrimitiveRequest};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let cache_path = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("kimi")
        .join("token.json");

    let client = LlmClient::from_preset("kimi")
        .expect("Preset should exist")
        .with_kimi_oauth(&cache_path)
        .build();

    let prompt = std::env::args().nth(1)
        .unwrap_or_else(|| "Hello! Please introduce yourself briefly.".to_string());

    let req = PrimitiveRequest::single_user_message(&prompt)
        .with_model("kimi-k2");

    println!("用户: {}\n", prompt);
    println!("AI:");
    let resp = client.complete(&req).await?;
    println!("{}", resp.content);

    Ok(())
}
