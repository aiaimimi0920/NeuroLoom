//! Kimi OAuth 白嫖通道环境测试
//!
//! ## 特性演示
//!
//! - 无需 API Key 即可启动！
//! - 自动触发后台针对 `auth.kimi.com` 的 Device Authorization 流程
//! - 唤醒浏览器以完成授权，并持久化 token 于临时工作目录
//!
//! 运行方式: cargo run -p nl_llm_v2 --example kimi_oauth_chat

use nl_llm_v2::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("========================================");
    println!("  🚀 Kimi OAuth 免密钥自动鉴权测试");
    println!("========================================\n");

    // 缓存文件存放在项目临时目录下以防污染
    let cache_file = std::env::temp_dir().join("neuro_loom_kimi_oauth_cache.json");

    // 注意：这里调用的是分离出来的专门应对 OAuth 流的 `kimi_oauth` 预设
    let client = LlmClient::from_preset("kimi_oauth")
        .expect("Kimi OAuth Preset should exist")
        .with_kimi_oauth(&cache_file)
        .build();

    let model = client.resolve_model("kimi-k2.5");
    println!("当前模型: {}\n", model);

    let prompt = "写一段关于人工智能探索宇宙的微型小说，30字以内。";
    println!("用户: {}\n", prompt);

    let req = PrimitiveRequest::single_user_message(prompt);

    // 调用 `complete` 遇到未授权或未获取 Token 时
    // 底层 Authenticator 会在终端打印要求并可能唤起浏览器，堵塞直至获取到合法 Token
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
