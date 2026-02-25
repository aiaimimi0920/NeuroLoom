//! AIGoCode 流式输出
//!
//! 演示流式响应和模型别名使用

use futures::StreamExt;
use nl_llm_v2::{LlmClient, PrimitiveRequest};
use std::io::Write;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("AIGOCODE_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: aigocode_stream <API_KEY>");
            std::process::exit(1);
        });

    let model_alias = std::env::args().nth(2).unwrap_or_else(|| "sonnet".to_string());
    let prompt = std::env::args().nth(3).unwrap_or_else(|| "Write a simple quicksort, no explanation".to_string());

    let client = LlmClient::from_preset("aigocode")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    println!("========================================");
    println!("  AIGoCode 流式输出");
    println!("========================================\n");
    println!("模型别名: {}", model_alias);
    println!("用户: {}\n", prompt);
    print!("AI: ");
    std::io::stdout().flush()?;

    // 使用模型别名进行流式请求
    let req = PrimitiveRequest::single_user_message(&prompt)
        .with_model(&model_alias);

    let mut stream = client.stream(&req).await?;
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(c) => {
                print!("{}", c.content);
                std::io::stdout().flush()?;
            }
            Err(e) => {
                eprintln!("\n读取流错误: {}", e);
                break;
            }
        }
    }
    println!("\n");

    // 提示可用别名
    println!("----------------------------------------");
    println!("  其他可用模型别名");
    println!("----------------------------------------");
    println!("  sonnet  - Claude Sonnet 4.5 (默认)");
    println!("  4o      - GPT-4o");
    println!("  gemini  - Gemini 2.0 Flash");
    println!("  r1      - DeepSeek R1 (推理增强)");

    Ok(())
}
