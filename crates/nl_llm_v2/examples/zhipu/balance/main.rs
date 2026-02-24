//! 智谱 AI 余额查询示例
//!
//! 运行: cargo run --example zhipu_balance

use nl_llm_v2::LlmClient;
use nl_llm_v2::PrimitiveRequest;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 从环境变量获取 API Key
    let api_key = std::env::var("ZHIPU_API_KEY")
        .or_else(|_| std::env::var("OPENAI_API_KEY"))
        .expect("请设置 ZHIPU_API_KEY 或 OPENAI_API_KEY 环境变量");

    let client = LlmClient::from_preset("zhipu")
        .expect("zhipu preset should exist")
        .with_api_key(&api_key)
        .with_concurrency()  // 启用并发控制
        .build();

    println!("=== 智谱 AI 余额查询 ===\n");

    // 查询余额
    match client.get_balance().await? {
        Some(balance) => println!("账户余额: {}", balance),
        None => println!("该平台不支持余额查询"),
    }

    // 查看并发状态
    if let Some(snapshot) = client.concurrency_snapshot() {
        println!("\n=== 并发状态 ===");
        println!("官方最大并发: {}", snapshot.official_max);
        println!("当前限制: {}", snapshot.current_limit);
        println!("活跃请求: {}", snapshot.active_requests);
    }

    // 发送一个测试请求
    println!("\n=== 测试请求 ===");
    let req = PrimitiveRequest::single_user_message("你好，请用一句话介绍自己。");
    let response = client.complete(&req).await?;
    println!("AI: {}", response.content);

    // 查看指标
    let metrics = client.metrics_summary();
    println!("\n=== 运行时指标 ===");
    println!("{}", metrics.format());

    Ok(())
}
