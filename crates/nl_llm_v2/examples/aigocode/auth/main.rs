//! AIGoCode 认证验证测试
//!
//! 演示如何验证 API 密钥和获取模型列表

use nl_llm_v2::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("AIGOCODE_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: aigocode_auth <API_KEY>");
            std::process::exit(1);
        });

    println!("========================================");
    println!("  AIGoCode 认证验证");
    println!("========================================\n");
    println!("网关: https://api.aigocode.com/v1");

    let client = LlmClient::from_preset("aigocode")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    println!("\n可用模型:");
    match client.list_models().await {
        Ok(models) => {
            for m in &models {
                println!("  • {} — {}", m.id, m.description);
            }
        }
        Err(e) => println!("  获取失败: {}", e),
    }

    println!("\n尝试基础通信 (claude-sonnet-4-5-20250929)...");
    let req = PrimitiveRequest::single_user_message("Say 'auth ok' in exactly 2 words");
    match client.complete(&req).await {
        Ok(resp) => {
            println!("\n认证通讯成功!");
            println!("模型响应: {}", resp.content);
        }
        Err(e) => {
            println!("\n认证通讯失败: {} (额度不足为预期行为)", e);
        }
    }

    // 测试模型别名
    println!("\n测试模型别名 '4o'...");
    let req = PrimitiveRequest::single_user_message("Say 'gpt ok'")
        .with_model("4o");
    match client.complete(&req).await {
        Ok(resp) => {
            println!("GPT-4o 响应: {}", resp.content);
        }
        Err(e) => {
            println!("GPT-4o 请求失败: {}", e);
        }
    }

    Ok(())
}
