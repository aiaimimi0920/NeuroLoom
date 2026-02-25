//! Cloudflare Workers AI 认证验证测试
//!
//! ## 特性演示
//!
//! - API Token 认证验证
//! - 并发控制
//! - 错误诊断
//!
//! 运行方式:
//!   方式1: cargo run -p nl_llm_v2 --example cloudflare_auth -- <api_token>
//!   方式2: 设置 CLOUDFLARE_API_TOKEN 环境变量

use nl_llm_v2::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("CLOUDFLARE_API_TOKEN")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: cloudflare_auth <API_TOKEN>");
            eprintln!("或设置 CLOUDFLARE_API_TOKEN 环境变量");
            std::process::exit(1);
        });

    println!("========================================");
    println!("  Cloudflare Workers AI 认证验证");
    println!("========================================\n");

    // 显示 API Token（脱敏）
    if api_key.len() > 8 {
        println!("API Token: {}...{}",
            &api_key[..4],
            &api_key[api_key.len().saturating_sub(4)..]);
    } else {
        println!("API Token: {} (过短)", api_key);
    }

    // 创建客户端（启用并发控制）
    let client = LlmClient::from_preset("cloudflare")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .with_concurrency()
        .build();

    println!("\n正在验证认证...");

    // 发送一个简单请求来验证认证
    let req = PrimitiveRequest::single_user_message("Say 'auth ok' in exactly 2 words")
        .with_model("llama-8b");  // 使用免费模型

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
            if let Some(snapshot) = client.concurrency_snapshot() {
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
            println!("  1. API Token 无效或已过期");
            println!("  2. 网络连接问题");
            println!("  3. Cloudflare 服务暂时不可用");
            println!("  4. Account ID 配置不正确");
            println!("\n获取凭据:");
            println!("  1. Account ID: Cloudflare Dashboard 右侧");
            println!("  2. API Token: https://dash.cloudflare.com/profile/api-tokens");
            std::process::exit(1);
        }
    }

    Ok(())
}
