use anyhow::Result;
use nl_llm_v2::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> Result<()> {
    let api_key = std::env::var("SOPHNET_API_KEYS").unwrap_or_else(|_| "dummy_credential".to_string());

    let client = LlmClient::from_preset("sophnet")
        .expect("Preset should exist")
        .with_api_key(api_key)
        .build();

    let req = PrimitiveRequest::single_user_message("Hello, please reply to me in Chinese and provide a short and funny joke!");

    println!("Sending request to SophNet...");
    
    let response = client.complete(&req).await?;
    println!("Response:\n{}", response.content);

    Ok(())
}
