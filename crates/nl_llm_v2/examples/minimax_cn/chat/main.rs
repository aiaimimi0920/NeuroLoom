//! MiniMax 中国站基础对话

use nl_llm_v2::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("MINIMAX_CN_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| "invalid_dummy_key_for_testing".to_string());

    let client = LlmClient::from_preset("minimax_cn")
        .expect("Preset should exist")
        .with_api_key(api_key)
        .build();

    let prompt = std::env::args().nth(2).unwrap_or_else(|| "用一句话介绍一下MiniMax。".to_string());

    println!("========================================");
    println!("  MiniMax 中国站对话");
    println!("========================================\n");
    println!("网关: https://api.minimaxi.com/v1");
    println!("模型: MiniMax-M2.5 (别名: minimax, m2.5)");
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
        Err(e) => { eprintln!("请求失败: {}", e); std::process::exit(1); }
    }
    Ok(())
}
