//! Codex API 流式输出测试（API Key 模式）

use nl_llm_v2::{LlmClient, PrimitiveRequest};
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("OPENAI_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: codex_api_stream <API_KEY> [prompt]");
            std::process::exit(1);
        });

    let prompt = std::env::args().nth(2)
        .unwrap_or_else(|| "Hello! Tell me a short story.".to_string());

    let client = LlmClient::from_preset("codex_api")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    let req = PrimitiveRequest::single_user_message(&prompt)
        .with_model("gpt-5.1-codex");

    println!("用户: {}\n", prompt);
    println!("AI (Stream):");

    let mut stream = client.stream(&req).await?;
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(c) => print!("{}", c.content),
            Err(e) => {
                eprintln!("\n❌ 流式错误: {}", e);
                break;
            }
        }
    }
    println!("\n");

    Ok(())
}
