//! Gemini CLI (Code Assist) 认证测试
//!
//! 验证是否能够读取 ~/AppData/Roaming/gcloud/application_default_credentials.json (或者对于legacy tier的特定路径)
//! 如果没有 Token 或者 Token 已过期，将提示执行 gcloud auth 登录。

use nl_llm_new::auth::providers::gemini_cli::GeminiCliOAuth;
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    println!("========================================");
    println!("  Gemini CLI Auth Check (nl_llm_new)");
    println!("========================================");

    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    
    // Gemini CLI path: %APPDATA%/gcloud/application_default_credentials.json
    let token_path = PathBuf::from(std::env::var("APPDATA").unwrap_or_else(|_| format!("{}/AppData/Roaming", home)))
        .join("gcloud")
        .join("application_default_credentials.json");

    println!("使用 Token 路径: {:?}", token_path);

    let mut auth = match GeminiCliOAuth::from_file(&token_path) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("初始化 Auth 失败: {:?}", e);
            return;
        }
    };

    println!("正在验证认证状态并自动刷新...");
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
