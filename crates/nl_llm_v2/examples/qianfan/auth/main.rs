//! 百度千帆 认证验证测试
//!
//! ## 特性演示
//!
//! - API Key 认证验证
//! - 并发控制
//! - 错误诊断
//!
//! 运行方式:
//!   方式1: cargo run -p nl_llm_v2 --example qianfan_auth -- <api_key>
//!   方式2: 设置 QIANFAN_API_KEY 环境变量

use nl_llm_v2::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("QIANFAN_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: qianfan_auth <API_KEY>");
            eprintln!("或设置 QIANFAN_API_KEY 环境变量");
            std::process::exit(1);
        });

    println!("========================================");
    println!("  百度千帆 认证验证");
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
    let client = LlmClient::from_preset("qianfan")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .with_concurrency()
        .build();

    println!("\n正在验证认证...");

    // 发送一个简单请求来验证认证（使用免费模型）
    let req = PrimitiveRequest::single_user_message("说'认证成功'这四个字")
        .with_model("lite");  // 使用免费模型

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
            println!("  1. API Key 无效或已过期");
            println!("  2. 网络连接问题");
            println!("  3. 百度千帆服务暂时不可用");
            println!("\n获取密钥:");
            println!("  1. 注册百度智能云: https://cloud.baidu.com");
            println!("  2. 进入千帆大模型平台: https://qianfan.cloud.baidu.com");
            println!("  3. 创建应用 → 获取 API Key");
            std::process::exit(1);
        }
    }

    Ok(())
}
