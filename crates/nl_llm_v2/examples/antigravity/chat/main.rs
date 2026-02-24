//! antigravity 平��测试 - chat
//!
//! 运行方式: cargo run --example antigravity_chat
//! 或直接运行: test.bat
//!
//! [修复] with_antigravity_oauth 需要缓存文件路径，不是 api_key
//! Antigravity 使用 OAuth 认证，会自动打开浏览器进行授权

use nl_llm_v2::{LlmClient, PrimitiveRequest};
use anyhow::Result;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // [修复] 使用编译期 CARGO_MANIFEST_DIR 定位缓存路径
    // current_exe() 不可靠：CARGO_TARGET_DIR 可能设在 temp 目录
    let cache_path: PathBuf = std::env::var("ANTIGRAVITY_CACHE_PATH").ok()
        .or_else(|| args.get(1).cloned())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            path.push("examples");
            path.push("antigravity");
            path.push(".cache");
            path.push("oauth_token.json");
            path
        });

    // 确保缓存目录存在
    if let Some(parent) = cache_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    println!("缓存文件: {}\n", cache_path.display());

    let client = LlmClient::from_preset("antigravity")
        .expect("Preset should exist")
        .with_antigravity_oauth(&cache_path)
        .build();

    let prompt = args.get(2).cloned()
        .unwrap_or_else(|| "Hello!".to_string());

    let req = PrimitiveRequest::single_user_message(&prompt)
        .with_model("gemini-2.5-flash");

    println!("用户: {}\n", prompt);
    println!("AI:");

    let resp = client.complete(&req).await?;
    println!("{}", resp.content);

    Ok(())
}
