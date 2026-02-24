//! DeepSeek 余额查询示例
//!
//! ## 特性演示
//!
//! - 余额查询（账户余额、赠送余额、充值余额）
//! - 并发控制
//! - 运行时指标收集
//!
//! 运行: cargo run -p nl_llm_v2 --example deepseek_balance

use nl_llm_v2::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 从环境变量获取 API Key
    let api_key = std::env::var("DEEPSEEK_API_KEY")
        .expect("请设置 DEEPSEEK_API_KEY 环境变量");

    println!("========================================");
    println!("  DeepSeek 余额查询 + 运行时演示");
    println!("========================================\n");

    let client = LlmClient::from_preset("deepseek")
        .expect("deepseek preset should exist")
        .with_api_key(&api_key)
        .with_concurrency()  // 启用并发控制
        .build();

    // ========== 查询余额 ==========
    println!("=== 账户余额 ===\n");

    match client.get_balance().await? {
        Some(balance) => println!("{}\n", balance),
        None => println!("该平台不支持余额查询\n"),
    }

    // ========== 查看并发状态 ==========
    if let Some(snapshot) = client.concurrency_snapshot() {
        println!("=== 并发配置 ===");
        println!("官方最大并发: {}", snapshot.official_max);
        println!("当前限制: {}", snapshot.current_limit);
        println!("初始状态: 就绪\n");
    }

    // ========== 发送测试请求 ==========
    println!("=== 测试请求 ===");
    let req = PrimitiveRequest::single_user_message("你好，请用一句话介绍自己。")
        .with_model("ds");  // 使用别名

    let response = client.complete(&req).await?;
    println!("用户: 你好，请用一句话介绍自己。");
    println!("AI: {}\n", response.content);

    if let Some(usage) = &response.usage {
        println!("Token 使用: prompt={}, completion={}, total={}",
            usage.prompt_tokens, usage.completion_tokens, usage.total_tokens);
    }

    // ========== 查看运行时指标 ==========
    let metrics = client.metrics_summary();
    println!("\n=== 运行时指标 ===");
    println!("总请求数: {}", metrics.total_requests);
    println!("平均延迟: {}ms", metrics.avg_latency_ms);
    println!("成功率: {:.1}%", metrics.success_rate * 100.0);

    // 显示并发控制器最终状态
    if let Some(snapshot) = client.concurrency_snapshot() {
        println!("\n=== 并发状态（最终）===");
        println!("当前限制: {}", snapshot.current_limit);
        println!("成功请求: {}", snapshot.success_count);
        if let Some(latency) = snapshot.avg_latency_ms {
            println!("平均延迟: {}ms", latency);
        }
    }

    Ok(())
}
