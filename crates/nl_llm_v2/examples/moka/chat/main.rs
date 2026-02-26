use anyhow::Result;
use nl_llm_v2::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> Result<()> {
    // MokaAI endpoints use standard authentication via Bearer tokens
    let api_key = std::env::var("MOKA_API_KEY")
        .expect("MOKA_API_KEY 环境变量未设置");

    let client = LlmClient::from_preset("moka")
        .expect("找不到 moka 预设")
        .with_api_key(api_key)
        .build();

    println!("Sending chat completion to MokaAI...");

    // Sending a standard payload
    let req = PrimitiveRequest::single_user_message("你好，请问你是谁？");
    
    let response = client.complete(&req).await?;
    
    println!("\nAI: {}\n", response.content);
    if let Some(usage) = response.usage {
        println!("Tokens used: Prompt={}, Completion={}, Total={}", usage.prompt_tokens, usage.completion_tokens, usage.total_tokens);
    }

    Ok(())
}
