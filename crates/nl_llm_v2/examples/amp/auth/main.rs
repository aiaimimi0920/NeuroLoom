//! Sourcegraph Amp 认证验证测试
//!
//! Amp (ampcode.com) 使用 API Key 认证（Bearer Token）。
//! 此示例验证 API Key 是否有效。
//!
//! ## 认证方式
//!
//! Amp 使用标准的 Bearer Token 认证：
//! ```http
//! Authorization: Bearer <AMP_API_KEY>
//! ```
//!
//! 运行方式:
//!   cargo run -p nl_llm_v2 --example amp_auth -- <api_key>
//! 或设置 AMP_API_KEY 环境变量后:
//!   cargo run -p nl_llm_v2 --example amp_auth

use nl_llm_v2::{LlmClient, PrimitiveRequest};
use nl_llm_v2::concurrency::ConcurrencyConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("AMP_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: amp_auth <API_KEY>");
            eprintln!("或设置 AMP_API_KEY 环境变量");
            std::process::exit(1);
        });

    println!("========================================");
    println!("  Amp CLI 认证验证");
    println!("========================================\n");

    // 显示 API Key（脱敏）
    if api_key.len() > 8 {
        println!("API Key: {}...{}",
            &api_key[..4],
            &api_key[api_key.len().saturating_sub(4)..]);
    } else {
        println!("API Key: {} (过短)", api_key);
    }

    // 创建客户端（启用并发控制）
    let client = LlmClient::from_preset("amp")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .with_concurrency_config(ConcurrencyConfig {
            official_max: 15,
            initial_limit: 5,
            min_limit: 1,
            max_limit: 15,
            ..Default::default()
        })
        .build();

    println!("\n正在验证认证...");

    // 发送一个简单请求来验证认证
    let req = PrimitiveRequest::single_user_message("Say 'auth ok' in exactly 2 words")
        .with_model("cheap");  // 使用低成本模型

    match client.complete(&req).await {
        Ok(resp) => {
            println!("\n✅ 认证成功！\n");
            println!("模型响应: {}", resp.content);

            // 显示详细使用信息
            if let Some(usage) = &resp.usage {
                println!("\nToken 使用统计:");
                println!("  输入 tokens: {}", usage.prompt_tokens);
                println!("  输出 tokens: {}", usage.completion_tokens);
                println!("  总计 tokens: {}", usage.total_tokens);
            }

            // 显示并发状态
            if let Some(ctrl) = client.concurrency_controller() {
                let snapshot = ctrl.snapshot();
                println!("\n并发状态:");
                println!("  成功请求: {}", snapshot.success_count);
                if let Some(latency) = snapshot.avg_latency_ms {
                    println!("  平均延迟: {}ms", latency);
                }
            }
        }
        Err(e) => {
            println!("\n❌ 认证失败: {}", e);
            println!("\n可能的原因:");
            println!("  1. API Key 无效或已过期");
            println!("  2. 网络连接问题");
            println!("  3. Amp 服务暂时不可用");
            std::process::exit(1);
        }
    }

    Ok(())
}
