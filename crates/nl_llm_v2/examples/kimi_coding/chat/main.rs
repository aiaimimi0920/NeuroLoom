//! Kimi For Coding (开发专供版) 基础对话测试
//!
//! ## 特性演示
//!
//! - Kimi For Coding 专用 API 节点 (`api.kimi.com`)
//! - 首选默认代码模型 `kimi-for-coding` 的无缝接入
//!
//! 运行方式:
//!   方式1: cargo run -p nl_llm_v2 --example kimi_coding_chat -- <api_key> [prompt]
//!   方式2: 使用 test.bat（自动读取 .env.local 中的密钥，或通过 blank 触发无密钥拦截演示）

use nl_llm_v2::{LlmClient, PrimitiveRequest, Capability};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("KIMI_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: kimi_coding_chat <API_KEY> [prompt]");
            std::process::exit(1);
        });

    let prompt = std::env::args().nth(2)
        .unwrap_or_else(|| "写一个简单的 Rust 快速排序，只给代码。".to_string());

    println!("========================================");
    println!("  Kimi For Coding 编程环境测试");
    println!("========================================\n");

    // 注意：这里使用的是分离出来的 `kimi_coding` 平台预设
    let client = LlmClient::from_preset("kimi_coding")
        .expect("Kimi Coding Preset should exist")
        .with_api_key(&api_key)
        .build();

    let default_model = client.resolve_model("coding");
    println!("当前生效判定模型 (默认): {}", default_model);
    
    let (input, output) = client.context_window_hint(&default_model);
    println!("代码上下文极限: {} Tokens (Input: {})", input + output, input);

    println!("----------------------------------------");
    println!("用户: {}\n", prompt);

    // Kimi Coding Preset 默认绑定的即为代码模型，所以无需显式 .with_model()
    let req = PrimitiveRequest::single_user_message(&prompt);

    match client.complete(&req).await {
        Ok(resp) => {
            println!("AI:\n{}\n", resp.content);
        }
        Err(e) => {
            println!("❌ 对话请求失败: {}", e);
            println!("\n提示: 这是专属路线 `api.kimi.com`。如果没有 Key 会由于 401 被拒绝访问。");
        }
    }

    Ok(())
}
