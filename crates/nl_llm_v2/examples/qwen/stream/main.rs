//! 通义千问 (Qwen) 流式输出测试
//!
//! ## 特性演示
//!
//! - 流式输出
//! - 并发控制
//! - 运行时指标收集
//!
//! 运行方式:
//!   方式1: cargo run -p nl_llm_v2 --example qwen_stream -- <api_key> [prompt]
//!   方式2: 设置 QWEN_API_KEY 环境变量后直接运行
//!   方式3: 使用 test.bat（自动读取 .env.local 中的密钥）

use nl_llm_v2::{LlmClient, PrimitiveRequest, ConcurrencyConfig};
use futures::StreamExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("QWEN_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: qwen_stream <API_KEY> [prompt]");
            eprintln!("或设置 QWEN_API_KEY 环境变量");
            std::process::exit(1);
        });

    let prompt = std::env::args().nth(2)
        .unwrap_or_else(|| "用三句话介绍一下 Rust 语言。".to_string());

    // ========== 创建带并发控制的客户端 ==========
    println!("========================================");
    println!("  通义千问 (Qwen) 流式输出 + 并发控制演示");
    println!("========================================\n");

    let client = LlmClient::from_preset("qwen")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        // 启用并发控制（使用自定义配置）
        .with_concurrency_config(ConcurrencyConfig::new(20)  // 阿里云常规最大并发
            .with_initial_limit(5))  // 初始并发限制
        .build();

    // 检查并发控制器状态
    if let Some(snapshot) = client.concurrency_snapshot() {
        println!("并发配置: {}/{} (初始/官方最大)", snapshot.current_limit, snapshot.official_max);
    }

    // ========== 使用 "plus" 别名发送流式请求 ==========
    let model_alias = "plus";  // 解析为 qwen-plus
    println!("\n模型: {} ({})", model_alias, client.resolve_model(model_alias));
    println!("用户: {}\n", prompt);
    print!("AI: ");

    let req = PrimitiveRequest::single_user_message(&prompt)
        .with_model(model_alias);

    let mut stream = client.stream(&req).await?;
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(c) => print!("{}", c.content),
            Err(e) => {
                eprintln!("\n流式错误: {}", e);
                break;
            }
        }
    }
    println!("\n");

    // ========== 显示运行时指标 ==========
    println!("----------------------------------------");
    println!("运行时指标:");
    let summary = client.metrics_summary();
    println!("  总请求数: {}", summary.total_requests);
    println!("  平均延迟: {}ms", summary.avg_latency_ms);
    println!("  成功率: {:.1}%", summary.success_rate * 100.0);

    // 显示并发控制器最终状态
    if let Some(snapshot) = client.concurrency_snapshot() {
        println!("\n并发状态:");
        println!("  当前限制: {}", snapshot.current_limit);
        println!("  活跃请求: {}", snapshot.active_requests);
        println!("  成功/失败: {}/{}", snapshot.success_count, snapshot.failure_count);
        if let Some(latency) = snapshot.avg_latency_ms {
            println!("  平均延迟: {}ms", latency);
        }
    }

    Ok(())
}
