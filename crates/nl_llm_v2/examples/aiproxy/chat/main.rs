use anyhow::Result;
use nl_llm_v2::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> Result<()> {
    // Aiproxy explicitly relies on the API Key
    let api_key = std::env::var("AIPROXY_API_KEY").expect("AIPROXY_API_KEY 环境变量未设置");

    // Initialize the AI Proxy preset client
    let client = LlmClient::from_preset("aiproxy")
        .expect("找不到 aiproxy 预设")
        .with_api_key(api_key)
        .build();

    println!("Sending chat completion to AI Proxy...");

    // Sending a simple message
    let req = PrimitiveRequest::single_user_message("你好，请问你是谁？能够背一首古诗吗");

    let response = client.complete(&req).await?;

    println!("\nAI: {}\n", response.content);
    if let Some(usage) = response.usage {
        println!(
            "Tokens used: Prompt={}, Completion={}, Total={}",
            usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
        );
    }

    Ok(())
}
