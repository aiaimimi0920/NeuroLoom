//! Sourcegraph Amp 流式输出测试
//!
//! Amp (ampcode.com) 是一个 AI 编码助手平台，聚合多个后端供应商。
//!
//! ## 特性演示
//!
//! - 流式输出
//! - 并发控制
//! - 运行时指标收集
//!
//! 运行方式:
//!   cargo run -p nl_llm_v2 --example amp_stream -- <api_key> [prompt]
//! 或设置 AMP_API_KEY 环境变量后:
//!   cargo run -p nl_llm_v2 --example amp_stream

use nl_llm_v2::{LlmClient, PrimitiveRequest};
use nl_llm_v2::concurrency::ConcurrencyConfig;
use futures::StreamExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("AMP_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: amp_stream <API_KEY> [prompt]");
            eprintln!("或设置 AMP_API_KEY 环境变量");
            std::process::exit(1);
        });

    let prompt = std::env::args().nth(2)
        .unwrap_or_else(|| "用三句话介绍一下 Rust 语言。".to_string());

    // ========== 创建带并发控制的客户端 ==========
    println!("========================================");
    println!("  Amp 流式输出 + 并发控制演示");
    println!("========================================\n");

    let client = LlmClient::from_preset("amp")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        // 启用并发控制（使用自定义配置）
        .with_concurrency_config(ConcurrencyConfig {
            official_max: 15,        // Amp 平台建议并发数
            initial_limit: 5,        // 初始并发限制
            min_limit: 1,            // 最小并发
            max_limit: 15,           // 最大并发
            ..Default::default()
        })
        .build();

    // 检查并发控制器状态
    if let Some(ctrl) = client.concurrency_controller() {
        let snapshot = ctrl.snapshot();
        println!("并发配置: {}/{} (初始/官方最大)", snapshot.current_limit, snapshot.official_max);
    }

    // ========== 使用 "cheap" 别名发送流式请求 ==========
    let model_alias = "cheap";  // 解析为 gpt-4o-mini
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
    if summary.total_requests > 0 {
        let error_rate = summary.total_errors as f64 / summary.total_requests as f64 * 100.0;
        println!("  错误率: {:.2}%", error_rate);
    }

    // 显示并发控制器最终状态
    if let Some(ctrl) = client.concurrency_controller() {
        let snapshot = ctrl.snapshot();
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
