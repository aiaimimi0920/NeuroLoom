use anyhow::Result;
use nl_llm_v2::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> Result<()> {
    let api_key = std::env::var("MISTRAL_API_KEY")
        .expect("MISTRAL_API_KEY 环境变量未设置");

    let client = LlmClient::from_preset("mistral")
        .expect("找不到 Mistral 预设")
        .with_api_key(api_key)
        .build();

    let model = "open-mistral-7b"; // You can use mistral-large-latest or other available models
    
    let prompt = "Why is the sky blue?";
    println!("Sending message to Mistral...");
    
    let req = PrimitiveRequest::single_user_message(prompt)
        .with_model(model);
        
    let response = client.complete(&req).await?;

    println!("\nResponse:");
    println!("{}", response.content);

    Ok(())
}
