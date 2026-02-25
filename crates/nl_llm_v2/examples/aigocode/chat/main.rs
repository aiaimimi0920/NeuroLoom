//! AIGoCode 基础对话
//!
//! 演示基本的对话功能和模型别名使用

use nl_llm_v2::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("AIGOCODE_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: aigocode_chat <API_KEY>");
            std::process::exit(1);
        });

    let model_alias = std::env::args().nth(2).unwrap_or_else(|| "sonnet".to_string());
    let prompt = std::env::args().nth(3).unwrap_or_else(|| "Briefly introduce yourself.".to_string());

    let client = LlmClient::from_preset("aigocode")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    println!("========================================");
    println!("  AIGoCode 基础对话");
    println!("========================================\n");
    println!("模型别名: {}", model_alias);
    println!("用户: {}\n", prompt);

    // 使用模型别名
    let req = PrimitiveRequest::single_user_message(&prompt)
        .with_model(&model_alias);

    match client.complete(&req).await {
        Ok(resp) => {
            println!("AI: {}", resp.content);
            if let Some(usage) = resp.usage {
                println!("\n[Token 用量: prompt={}, completion={}, total={}]",
                    usage.prompt_tokens, usage.completion_tokens, usage.total_tokens);
            }
            if let Some(model) = resp.model {
                println!("[实际模型: {}]", model);
            }
        }
        Err(e) => {
            eprintln!("请求失败: {} (额度不足为预期行为)", e);
        }
    }

    // 演示多种模型别名
    println!("\n----------------------------------------");
    println!("  可用模型别名");
    println!("----------------------------------------");
    let aliases = [
        ("sonnet / claude", "claude-sonnet-4-5-20250929"),
        ("4o", "gpt-4o"),
        ("4o-mini", "gpt-4o-mini"),
        ("gemini", "gemini-2.0-flash"),
        ("deepseek", "deepseek-chat"),
        ("r1", "deepseek-reasoner"),
    ];
    for (alias, model) in aliases {
        println!("  '{}' -> {}", alias, model);
    }

    Ok(())
}
