//! Qwen2.5-Coder (阿里极速编程模型) 基础流式请求测试
//!
//! 运行方式:
//!   方式1: cargo run -p nl_llm_v2 --example qwen_coder_stream -- <api_key> [prompt]
//!   方式2: 使用 test.bat

use nl_llm_v2::{LlmClient, PrimitiveRequest};
use futures::StreamExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("QWEN_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("需要提供 QwEN_API_KEY");
            std::process::exit(1);
        });

    let prompt = std::env::args().nth(2)
        .unwrap_or_else(|| "写一段简单的冒泡排序，不要解释".to_string());

    println!("========================================");
    println!("  Qwen2.5-Coder 代码流式输出");
    println!("========================================\n");

    let client = LlmClient::from_preset("qwen_coder")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    println!("模型: {}", client.resolve_model("qwen_coder"));
    println!("用户: {}\n", prompt);
    print!("AI: \n");

    let req = PrimitiveRequest::single_user_message(&prompt);

    let mut stream = client.stream(&req).await?;
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(c) => print!("{}", c.content),
            Err(e) => {
                eprintln!("\n流式错误: {}", e);
                break;
            }
        }
    }
    println!("\n");

    Ok(())
}
