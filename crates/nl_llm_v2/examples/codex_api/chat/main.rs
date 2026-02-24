//! Codex API 对话测试（API Key 模式）

use nl_llm_v2::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("OPENAI_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: codex_api_chat <API_KEY> [prompt]");
            std::process::exit(1);
        });

    let prompt = std::env::args().nth(2)
        .unwrap_or_else(|| "Hello! Please introduce yourself briefly.".to_string());

    let client = LlmClient::from_preset("codex_api")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    let req = PrimitiveRequest::single_user_message(&prompt)
        .with_model("gpt-5.1-codex");

    println!("用户: {}\n", prompt);
    let resp = client.complete(&req).await?;
    println!("AI: {}\n", resp.content);

    Ok(())
}
