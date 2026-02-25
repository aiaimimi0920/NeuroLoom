//! Kimi (Moonshot) 认证验证测试
//!
//! ## 特性演示
//!
//! - API Key 认证验证
//! - 余额查询特性 (如果有相关权限)
//! - 错误诊断建议 (针对缺无 key 或者 401 报错提供完善提示)
//!
//! 运行方式:
//!   方式1: cargo run -p nl_llm_v2 --example kimi_auth -- <api_key>
//!   方式2: 设置 KIMI_API_KEY 后运行
//!   方式3: 使用 test.bat（自动读取 .env.local 中的密钥，没有也可以用来欣赏报错）

use nl_llm_v2::{LlmClient, PrimitiveRequest};
use nl_llm_v2::concurrency::ConcurrencyConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 允许随便传个废 key 来演示没钱或者鉴权失败的报错链条
    let api_key = std::env::var("KIMI_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| "invalid_dummy_key_for_testing".to_string());

    println!("========================================");
    println!("  Kimi (Moonshot) 认证验证");
    println!("========================================\n");

    if api_key == "invalid_dummy_key_for_testing" {
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

    // 创建客户端
    let client = LlmClient::from_preset("kimi")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .with_concurrency_config(ConcurrencyConfig {
            official_max: 20,
            initial_limit: 5,
            min_limit: 1,
            max_limit: 20,
            ..Default::default()
        })
        .build();

    println!("\n正在验证认证并尝试读取余额...");

    // 1. 验证余额功能 (ProviderExtension 扩展能力)
    match client.get_balance().await {
        Ok(Some(balance)) => {
            println!("✅ 财务账单成功获取:");
            println!("  {}\n", balance);
        }
        Ok(None) => println!("ℹ️ 当前不支持该站点的余额查询"),
        Err(e) => {
            println!("❌ 余额查询遭到拒绝: {}", e);
            println!("可能为 Key 无效，或者权限不足\n");
        }
    }

    println!("\n尝试基础通信...");
    // 2. 发送一个简单请求来验证认证
    let req = PrimitiveRequest::single_user_message("Say 'auth ok' in exactly 2 words")
        .with_model("k2.5");

    match client.complete(&req).await {
        Ok(resp) => {
            println!("\n✅ 认证通讯成功！\n");
            println!("模型响应: {}", resp.content);
        }
        Err(e) => {
            println!("\n❌ 认证通讯失败: {}", e);
            println!("\n可能的原因:");
            println!("  1. API Key 无效、拼写错误，或者欠费已停机（例如 401 Unauthorized / 402 Payment Required）");
            println!("  2. 网络连接到了 `api.moonshot.cn` 但是遭到了拦截");
            println!("\n您可以前往获取有效 API Key: https://platform.moonshot.cn/console/api-keys");
            
            // 作为缺少密钥情况下的"成功退出"，演示目的已达到
            if api_key == "invalid_dummy_key_for_testing" {
                println!("\n(测试预期：本次作为错误演示顺利结束)");
                std::process::exit(0);
            } else {
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
