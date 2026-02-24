//! Claude OAuth 对话示例

use nl_llm_v2::{LlmClient, PrimitiveRequest};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let cache_path = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("anthropic")
        .join("token.json");

    let client = LlmClient::from_preset("claude_oauth")
        .expect("Preset should exist")
        .with_claude_oauth(&cache_path)
        .build();

    let prompt = std::env::args().nth(1)
        .unwrap_or_else(|| "Hello! Please introduce yourself briefly.".to_string());

    let req = PrimitiveRequest::single_user_message(&prompt)
        .with_model("claude-sonnet");

    println!("用户: {}\n", prompt);
    let resp = client.complete(&req).await?;
    println!("AI: {}", resp.content);

    Ok(())
}
