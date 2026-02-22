//! Antigravity (Gemini Code Assist) 认证测试
//!
//! 验证是否能够读取 ~/.nl_llm/antigravity_token.json
//! 如果没有 Token 或者 Token 已过期，将自动弹出浏览器执行 OAuth 登录。

use nl_llm_new::auth::providers::antigravity::AntigravityOAuth;
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    println!("========================================");
    println!("  Antigravity Auth Check (nl_llm_new)");
    println!("========================================");

    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    let token_path = PathBuf::from(home).join(".nl_llm").join("antigravity_token.json");

    println!("使用 Token 路径: {:?}", token_path);

    let mut auth = match AntigravityOAuth::from_file(&token_path) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("初始化 Auth 失败: {:?}", e);
            return;
        }
    };

    println!("正在验证认证状态并自动登录...");
    match auth.ensure_authenticated().await {
        Ok(_) => {
            println!("===== 认证成功! =====");
            if let Some(token) = auth.access_token() {
                println!("Access Token (前60字符): {}...", &token[..60.min(token.len())]);
            }
        }
        Err(e) => {
            println!("===== 认证失败 =====");
            println!("Error: {:?}", e);
        }
    }
}
