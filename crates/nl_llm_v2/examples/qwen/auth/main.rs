//! Qwen OAuth 认证测试
//!
//! 通过 Device Code + PKCE 完成 Qwen OAuth 登录。
//! 首次运行会打开浏览器进行授权。

use nl_llm_v2::{LlmClient, PrimitiveRequest};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let cache_path = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("qwen")
        .join("token.json");

    println!("=== Qwen OAuth 认证测试 ===");
    println!("Token 缓存: {}\n", cache_path.display());

    let client = LlmClient::from_preset("qwen")
        .expect("Preset should exist")
        .with_qwen_oauth(&cache_path)
        .build();

    // 发送一个简单请求来触发 OAuth 登录并验证认证是否成功
    let req = PrimitiveRequest::single_user_message("Hello! Say 'auth ok' if you can read this.")
        .with_model("qwen3-coder-plus");

    println!("发送测试请求...\n");

    let resp = client.complete(&req).await?;
    println!("AI: {}", resp.content);
    println!("\n✅ Qwen OAuth 认证成功！");

    Ok(())
}
