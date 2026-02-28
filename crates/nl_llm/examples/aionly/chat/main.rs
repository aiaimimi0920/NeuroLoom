use anyhow::Result;
use nl_llm::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> Result<()> {
    let api_key = std::env::var("AIONLY_API_KEY").expect("AIONLY_API_KEY 环境变量未设置");

    let client = LlmClient::from_preset("aionly")
        .expect("找不到 aionly 预设")
        .with_api_key(api_key)
        .build();

    println!("Sending chat completion to AiOnly...");

    let req = PrimitiveRequest::single_user_message("你好，请简单介绍一下你自己。");

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
