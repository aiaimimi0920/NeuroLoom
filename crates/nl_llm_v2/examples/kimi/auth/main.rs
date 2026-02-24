//! Kimi OAuth 认证测试
//!
//! 通过 Device Code 完成 Kimi OAuth 登录。
//! 首次运行会打开浏览器进行授权。

use nl_llm_v2::{LlmClient, PrimitiveRequest};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let cache_path = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("kimi")
        .join("token.json");

    println!("=== Kimi OAuth 认证测试 ===");
    println!("Token 缓存: {}\n", cache_path.display());

    let client = LlmClient::from_preset("kimi")
        .expect("Preset should exist")
        .with_kimi_oauth(&cache_path)
        .build();

    let req = PrimitiveRequest::single_user_message("Hello! Say 'auth ok' if you can read this.")
        .with_model("kimi-k2");

    println!("发送测试请求...\n");

    let resp = client.complete(&req).await?;
    println!("AI: {}", resp.content);
    println!("\n✅ Kimi OAuth 认证成功！");

    Ok(())
}
