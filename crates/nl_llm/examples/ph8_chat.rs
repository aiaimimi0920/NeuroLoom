use anyhow::Result;
use nl_llm::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> Result<()> {
    let api_key = std::env::var("PH8_API_KEYS").unwrap_or_else(|_| "dummy_credential".to_string());

    let client = LlmClient::from_preset("ph8")
        .expect("Preset should exist")
        .with_api_key(api_key)
        .build();

    let req = PrimitiveRequest::single_user_message("Hello, please reply to me in Chinese and provide a short and funny joke!").with_model("qwen-max");

    println!("Sending request to PH8 (model: qwen-max)...");
    
    let response = client.complete(&req).await?;
    println!("Response:\n{}", response.content);

    Ok(())
}
