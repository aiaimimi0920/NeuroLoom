//! AWS Claude AK/SK 模式 — 基础对话

use nl_llm_v2::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let access_key = std::env::var("AWS_ACCESS_KEY_ID")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: aws_claude_ak_chat <ACCESS_KEY_ID> <SECRET_ACCESS_KEY> [REGION]");
            std::process::exit(1);
        });
    let secret_key = std::env::var("AWS_SECRET_ACCESS_KEY")
        .or_else(|_| std::env::args().nth(2).ok_or(()))
        .unwrap_or_else(|_| { eprintln!("缺少 SECRET_ACCESS_KEY"); std::process::exit(1); });
    let region = std::env::var("AWS_REGION")
        .or_else(|_| std::env::args().nth(3).ok_or(()))
        .unwrap_or_else(|_| "us-east-1".to_string());

    let prompt = std::env::args().nth(4).unwrap_or_else(|| "用一句话介绍一下你自己".to_string());

    println!("========================================");
    println!("  AWS Claude AK/SK 模式 — 基础对话");
    println!("========================================\n");
    println!("Region: {}", region);
    println!("Access Key: {}...", &access_key[..8.min(access_key.len())]);
    println!("模型: anthropic.claude-sonnet-4-5 (默认)");
    println!("用户: {}\n", prompt);

    // 注意: AK/SK 模式需要 SigV4 签名的自定义 Authenticator
    // 这里仅展示框架结构，实际需要实现 SigV4 签名逻辑
    let client = LlmClient::from_preset("aws_claude_ak")
        .expect("Preset should exist")
        .with_api_key(&format!("{}:{}", access_key, secret_key))
        .build();

    let req = PrimitiveRequest::single_user_message(&prompt);
    match client.complete(&req).await {
        Ok(resp) => {
            println!("AI: {}", resp.content);
            if let Some(usage) = resp.usage {
                println!("\n[Token 用量: prompt={}, completion={}, total={}]",
                    usage.prompt_tokens, usage.completion_tokens, usage.total_tokens);
            }
        }
        Err(e) => { eprintln!("请求失败: {} (AK/SK 模式需要 SigV4 签名支持)", e); }
    }
    Ok(())
}
