use anyhow::Result;
use nl_llm::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> Result<()> {
    let api_key = std::env::var("JINA_API_KEY").expect("JINA_API_KEY 环境变量未设置");

    let client = LlmClient::from_preset("jina")
        .expect("找不到 Jina 预设")
        .with_api_key(api_key)
        .build();

    let model = "jina-embeddings-v3"; // Typically Jina is embeddings, but let's test a generic query if chat is supported or switch to embedding action natively if implemented later.

    println!("Sending message to Jina...");

    let prompt = "Hello, Jina!";
    let req = PrimitiveRequest::single_user_message(prompt).with_model(model);

    let response = client.complete(&req).await?;

    println!("\nResponse:");
    println!("{}", response.content);

    Ok(())
}
