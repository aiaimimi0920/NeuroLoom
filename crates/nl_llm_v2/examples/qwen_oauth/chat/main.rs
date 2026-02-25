//! Qwen OAuth 门户体验中心授权测试
//!
//! ## 特性演示
//!
//! - 无需 API Key 即可启动！
//! - 直接连接通义千问后台 `portal.qwen.ai` 走 Web OAuth 通道
//! - 在控制台打印 User Code 并唤醒浏览器，支持无缝 Token 刷新和本地文件缓存
//!
//! 运行方式: cargo run -p nl_llm_v2 --example qwen_oauth_chat

use nl_llm_v2::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("========================================");
    println!("  🚀 Qwen (通义千问) OAuth 免密钥自动鉴权测试");
    println!("========================================\n");

    let cache_file = std::env::temp_dir().join("neuro_loom_qwen_oauth_cache.json");

    // 注意：使用的是明确拆分出来的 `qwen_oauth` 预设体系
    let client = LlmClient::from_preset("qwen_oauth")
        .expect("Qwen OAuth Preset should exist")
        .with_qwen_oauth(&cache_file)
        .build();

    let model = client.resolve_model("qwen-plus");
    println!("当前模型: {}\n", model);

    let prompt = "用 Rust 写一个 Hello World，只需代码。";
    println!("用户: {}\n", prompt);

    let req = PrimitiveRequest::single_user_message(prompt);

    match client.complete(&req).await {
        Ok(resp) => {
            println!("AI:\n{}\n", resp.content);
        }
        Err(e) => {
            eprintln!("请求失败: {}", e);
        }
    }

    Ok(())
}
