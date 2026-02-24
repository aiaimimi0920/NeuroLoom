//! Codex API 认证测试（API Key 模式）

use nl_llm_v2::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() {
    let api_key = std::env::var("OPENAI_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: codex_api_auth <API_KEY>");
            eprintln!("  或设置环境变量 OPENAI_API_KEY");
            std::process::exit(1);
        });

    println!("=== Codex API 认证测试 ===");
    println!("Key: {}...{}\n", &api_key[..8], &api_key[api_key.len()-4..]);

    let client = LlmClient::from_preset("codex_api")
        .expect("Preset should exist")
        .with_api_key(&api_key)
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
            println!("\n✅ Codex API 认证成功！");
        }
        Err(e) => {
            eprintln!("❌ 请求失败: {}", e);
            std::process::exit(1);
        }
    }
}
