//! vertex_api 平台测试 - chat
//!
//! Vertex AI API Key 认证模式。
//!
//! 运行方式:
//!   cargo run --example vertex_api_chat -- <api_key> [prompt]
//! 或直接运行: test.bat

use nl_llm_v2::{LlmClient, PrimitiveRequest};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let api_key = std::env::var("VERTEX_API_KEY").ok()
        .or_else(|| args.get(1).cloned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            eprintln!("Error: 请提供 Vertex API Key");
            std::process::exit(1);
        });

    let client = LlmClient::from_preset("vertex_api")
        .expect("Preset should exist")
        .with_vertex_api_key(&api_key)
        .build();

    let prompt = args.get(2).cloned()
        .unwrap_or_else(|| "Hello! Please introduce yourself briefly.".to_string());

    let req = PrimitiveRequest::single_user_message(&prompt)
        .with_model("gemini-2.5-flash");

    println!("用户: {}\n", prompt);
    println!("AI:");

    let resp = client.complete(&req).await?;
    println!("{}", resp.content);

    Ok(())
}
