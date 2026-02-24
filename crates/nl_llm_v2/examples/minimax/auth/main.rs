//! MiniMax (en) 认证验证测试

use nl_llm_v2::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("MINIMAX_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| "invalid_dummy_key_for_testing".to_string());

    println!("========================================");
    println!("  MiniMax (en) 认证验证");
    println!("========================================\n");
    println!("网关: https://api.minimax.io/v1");

    if api_key.len() > 8 {
        println!("API Key: {}...{}", &api_key[..6], &api_key[api_key.len().saturating_sub(4)..]);
    }

    let client = LlmClient::from_preset("minimax")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    println!("\n可用模型:");
    match client.list_models().await {
        Ok(models) => { for m in &models { println!("  • {} — {}", m.id, m.description); } }
        Err(e) => println!("  获取失败: {}", e),
    }

    // 并发配置
    if let Some(ext) = client.extension() {
        let config = ext.concurrency_config();
        println!("\n并发配置:");
        println!("  官方上限: {}", config.official_max);
        println!("  初始并发: {}", config.initial_limit);
    }

    println!("\n尝试基础通信...");
    let req = PrimitiveRequest::single_user_message("Say 'auth ok' in exactly 2 words");
    match client.complete(&req).await {
        Ok(resp) => {
            println!("\n✅ 认证通讯成功！");
            println!("模型响应: {}", resp.content);
        }
        Err(e) => {
            println!("\n❌ 认证通讯失败: {}", e);
            println!("\n可前往: https://platform.minimax.io/subscribe/coding-plan");
            std::process::exit(1);
        }
    }
    Ok(())
}
