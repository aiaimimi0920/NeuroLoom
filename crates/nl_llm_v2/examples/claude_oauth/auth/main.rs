//! Claude OAuth 认证示例
//!
//! 通过浏览器完成 Claude OAuth (Authorization Code + PKCE)。
//! 首次运行会打开浏览器进行授权。

use nl_llm_v2::{LlmClient, PrimitiveRequest};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let cache_path = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("anthropic")
        .join("token.json");

    println!("=== Claude OAuth 认证测试 ===");
    println!("Token 缓存: {}\n", cache_path.display());

    let client = LlmClient::from_preset("claude_oauth")
        .expect("Preset should exist")
        .with_claude_oauth(&cache_path)
        .build();

    let req = PrimitiveRequest::single_user_message("Hello! Say 'auth ok' if you can read this.")
        .with_model("claude-sonnet");

    println!("发送测试请求...\n");

    let resp = client.complete(&req).await?;
    println!("AI: {}", resp.content);
    println!("\n✅ Claude OAuth 认证成功！");

    Ok(())
}
