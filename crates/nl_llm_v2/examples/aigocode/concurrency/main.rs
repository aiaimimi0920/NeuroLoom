//! AIGoCode 并发控制演示
//!
//! 演示如何启用并发控制，动态调整并发数

use nl_llm_v2::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("AIGOCODE_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: aigocode_concurrency <API_KEY>");
            std::process::exit(1);
        });

    println!("========================================");
    println!("  AIGoCode 并发控制");
    println!("========================================\n");

    // 启用并发控制
    let client = LlmClient::from_preset("aigocode")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .with_concurrency()  // 启用并发控制
        .build();

    println!("并发配置:");
    println!("  - 官方上限: 5");
    println!("  - 初始并发: 3");
    println!("  - 策略: AIMD 动态调整\n");

    // 获取并发状态
    if let Some(controller) = client.concurrency_controller() {
        let status = controller.status();
        println!("当前状态:");
        println!("  - 当前限制: {}", status.current_limit);
        println!("  - 活跃请求: {}", status.active_count);
        println!("  - 总请求数: {}", status.total_requests);
        println!("  - 成功数: {}", status.success_count);
        println!("  - 失败数: {}", status.failure_count);
    }

    println!("\n发送测试请求...\n");

    let req = PrimitiveRequest::single_user_message("Say 'concurrency test ok' in 3 words");
    match client.complete(&req).await {
        Ok(resp) => {
            println!("响应: {}", resp.content);
        }
        Err(e) => {
            println!("请求失败: {} (额度不足为预期行为)", e);
        }
    }

    // 再次查看并发状态
    if let Some(controller) = client.concurrency_controller() {
        let status = controller.status();
        println!("\n请求后状态:");
        println!("  - 当前限制: {}", status.current_limit);
        println!("  - 活跃请求: {}", status.active_count);
        println!("  - 成功数: {}", status.success_count);
    }

    Ok(())
}
