//! AICodeMirror 指标收集演示
//!
//! 演示如何启用和查看请求指标

use nl_llm_v2::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("AICODEMIRROR_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: aicodemirror_metrics <API_KEY>");
            std::process::exit(1);
        });

    println!("========================================");
    println!("  AICodeMirror 指标收集");
    println!("========================================\n");

    // 启用指标收集和并发控制
    let client = LlmClient::from_preset("aicodemirror")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .with_concurrency()
        .with_metrics()  // 启用指标收集
        .build();

    println!("已启用指标收集和并发控制\n");

    // 发送几个请求
    let prompts = [
        "Say 'test 1'",
        "Say 'test 2'",
        "Say 'test 3'",
    ];

    for (i, prompt) in prompts.iter().enumerate() {
        let req = PrimitiveRequest::single_user_message(prompt);
        match client.complete(&req).await {
            Ok(_) => println!("请求 {} 成功", i + 1),
            Err(_) => println!("请求 {} 失败", i + 1),
        }
    }

    println!("\n----------------------------------------");
    println!("  指标统计");
    println!("----------------------------------------\n");

    if let Some(metrics) = client.metrics() {
        let snapshot = metrics.snapshot();
        println!("请求统计:");
        println!("  - 总请求数: {}", snapshot.total_requests);
        println!("  - 成功数: {}", snapshot.success_count);
        println!("  - 失败数: {}", snapshot.failure_count);
        println!("  - 成功率: {:.1}%", snapshot.success_rate() * 100.0);

        println!("\n延迟统计:");
        println!("  - 平均延迟: {:.1}ms", snapshot.avg_latency_ms);
        println!("  - 最小延迟: {:.1}ms", snapshot.min_latency_ms);
        println!("  - 最大延迟: {:.1}ms", snapshot.max_latency_ms);

        println!("\nToken 统计:");
        println!("  - 总输入 Token: {}", snapshot.total_input_tokens);
        println!("  - 总输出 Token: {}", snapshot.total_output_tokens);
    } else {
        println!("指标未启用");
    }

    // 并发状态
    if let Some(controller) = client.concurrency_controller() {
        let status = controller.status();
        println!("\n并发状态:");
        println!("  - 当前限制: {}", status.current_limit);
        println!("  - 成功/失败: {}/{}", status.success_count, status.failure_count);
    }

    Ok(())
}
