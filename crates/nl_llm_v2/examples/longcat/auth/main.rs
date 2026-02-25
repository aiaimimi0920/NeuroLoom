//! Longcat AI 认证验证测试
//!
//! ## 特性演示
//!
//! - API Key 认证验证
//! - 并发控制
//! - 错误诊断建议
//!
//! 运行方式:
//!   方式1: cargo run -p nl_llm_v2 --example longcat_auth -- <api_key>
//!   方式2: 设置 LONGCAT_API_KEY 后运行
//!   方式3: 使用 test.bat

use nl_llm_v2::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("LONGCAT_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| "invalid_dummy_key_for_testing".to_string());

    let is_dummy_key = api_key == "invalid_dummy_key_for_testing";

    println!("========================================");
    println!("  Longcat (Longcat AI) 认证验证");
    println!("========================================\n");

    if is_dummy_key {
        println!("⚠️  注意: 未检测到有效 API Key, 当前使用测试桩 (预期将认证失败)\n");
    } else {
        if api_key.len() > 8 {
            println!("API Key: {}...{}",
                &api_key[..4],
                &api_key[api_key.len().saturating_sub(4)..]);
        } else {
            println!("API Key: {} (过短)", api_key);
        }
    }

    let client = LlmClient::from_preset("longcat")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .with_concurrency()  // 启用并发控制
        .build();

    // 1. 模型列表（静态，无需网络）
    println!("\n可用模型:");
    match client.list_models().await {
        Ok(models) => {
            for m in &models {
                println!("  • {} — {}", m.id, m.description);
            }
        }
        Err(e) => println!("  获取失败: {}", e),
    }

    // 2. 余额查询
    println!("\n正在查询余额...");
    match client.get_balance().await {
        Ok(Some(balance)) => println!("✅ 余额: {}", balance),
        Ok(None) => println!("ℹ️  该平台暂不支持余额查询"),
        Err(e) => println!("❌ 余额查询失败: {}", e),
    }

    // 3. 发送基础通信请求
    println!("\n尝试基础通信...");
    let req = PrimitiveRequest::single_user_message("Say 'auth ok' in exactly 2 words")
        .with_model("flash");  // 使用别名

    match client.complete(&req).await {
        Ok(resp) => {
            println!("\n✅ 认证通讯成功！\n");
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
            println!("\n❌ 认证通讯失败: {}", e);
            println!("\n可能的原因:");
            println!("  1. API Key 无效或配置不正确");
            println!("  2. 网络无法连接到 api.longcat.chat");
            println!("\n您可以前往获取 API Key: https://longcat.chat/platform/api_keys");

            if is_dummy_key {
                println!("\n(测试预期：本次作为错误演示顺利结束)");
                std::process::exit(0);
            } else {
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
