//! AWS Claude (Amazon Bedrock) 基础对话

use nl_llm_v2::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("AWS_BEDROCK_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: aws_claude_chat <API_KEY>");
            eprintln!("  或设置环境变量: AWS_BEDROCK_API_KEY");
            std::process::exit(1);
        });

    let prompt = std::env::args().nth(2).unwrap_or_else(|| "用一句话介绍一下你自己".to_string());

    let client = LlmClient::from_preset("aws_claude")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    println!("========================================");
    println!("  AWS Claude (Bedrock) 基础对话");
    println!("========================================\n");
    println!("Region: us-east-1");
    println!("模型: anthropic.claude-sonnet-4-6 (默认)");
    println!("用户: {}\n", prompt);

    let req = PrimitiveRequest::single_user_message(&prompt);
    match client.complete(&req).await {
        Ok(resp) => {
            println!("AI: {}", resp.content);
            if let Some(usage) = resp.usage {
                println!("\n[Token 用量: prompt={}, completion={}, total={}]",
                    usage.prompt_tokens, usage.completion_tokens, usage.total_tokens);
            }
        }
        Err(e) => { eprintln!("请求失败: {}", e); }
    }
    Ok(())
}
