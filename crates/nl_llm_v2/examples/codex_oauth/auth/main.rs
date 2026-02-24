//! Codex OAuth 认证测试（带详细错误输出）

use nl_llm_v2::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() {
    let cache_path = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("codex")
        .join("token.json");

    println!("=== Codex OAuth 认证测试 ===");
    println!("Token 缓存: {}\n", cache_path.display());

    // 检查是否已有 token
    if cache_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&cache_path) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(email) = v.get("email").and_then(|e| e.as_str()) {
                    println!("[DEBUG] 已有缓存 token, Email: {}", email);
                }
            }
        }
    }

    let client = LlmClient::from_preset("codex_oauth")
        .expect("Preset should exist")
        .with_codex_oauth(&cache_path)
        .build();

    let req = PrimitiveRequest::single_user_message("Hello! Say 'auth ok' if you can read this.")
        .with_model("codex");

    println!("发送测试请求 (model: codex → gpt-5.1-codex)...\n");

    match client.complete(&req).await {
        Ok(resp) => {
            println!("AI: {}", resp.content);
            println!("Model: {}", resp.model);
            if let Some(usage) = &resp.usage {
                println!("Usage: prompt={}, completion={}, total={}", 
                    usage.prompt_tokens, usage.completion_tokens, usage.total_tokens);
            }
            println!("\n✅ Codex OAuth 认证成功！");
        }
        Err(e) => {
            eprintln!("❌ 请求失败!");
            eprintln!("Error: {:?}", e);
            eprintln!("Error (display): {}", e);
            // 打印完整错误链
            let mut source = e.source();
            while let Some(cause) = source {
                eprintln!("  Caused by: {}", cause);
                source = std::error::Error::source(cause);
            }
            std::process::exit(1);
        }
    }
}
