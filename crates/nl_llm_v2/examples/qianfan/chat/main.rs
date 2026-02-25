//! 百度千帆 基础对话
//!
//! ## 特性演示
//!
//! - 基础对话
//! - 并发控制
//! - 运行时指标
//!
//! 运行方式:
//!   方式1: cargo run -p nl_llm_v2 --example qianfan_chat -- <api_key> [prompt]
//!   方式2: 设置 QIANFAN_API_KEY 环境变量

use nl_llm_v2::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("QIANFAN_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: qianfan_chat <API_KEY> [prompt]");
            eprintln!("或设置 QIANFAN_API_KEY 环境变量");
            std::process::exit(1);
        });
    let prompt = std::env::args().nth(2).unwrap_or_else(|| "用一句话介绍一下你自己".to_string());

    let client = LlmClient::from_preset("qianfan")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .with_concurrency()  // 启用并发控制
        .build();

    println!("========================================");
    println!("  百度千帆 基础对话");
    println!("========================================\n");
    println!("模型: ernie-4.5-turbo-128k (默认)");

    // 显示并发配置
    if let Some(snapshot) = client.concurrency_snapshot() {
        println!("并发: {}/{} (初始/最大)", snapshot.current_limit, snapshot.official_max);
    }
    println!("用户: {}\n", prompt);

    let req = PrimitiveRequest::single_user_message(&prompt);
    match client.complete(&req).await {
        Ok(resp) => {
            println!("AI: {}", resp.content);
            if let Some(usage) = resp.usage {
                println!("\n[Token 用量: prompt={}, completion={}, total={}]",
                    usage.prompt_tokens, usage.completion_tokens, usage.total_tokens);
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
        Err(e) => { eprintln!("请求失败: {}", e); }
    }
    Ok(())
}
