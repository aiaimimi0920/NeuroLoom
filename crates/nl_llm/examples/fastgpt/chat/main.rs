use anyhow::Result;
use nl_llm::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> Result<()> {
    let api_key = std::env::var("FASTGPT_API_KEY").expect("FASTGPT_API_KEY 环境变量未设置");

    let client = LlmClient::from_preset("fastgpt")
        .expect("找不到 FastGPT 预设")
        .with_api_key(api_key)
        .build();

    let model = "fastgpt-default";

    let prompt = "Hi, can you introduce yourself?";
    println!("Sending message to FastGPT...");

    let req = PrimitiveRequest::single_user_message(prompt).with_model(model);

    let response = client.complete(&req).await?;

    println!("\nResponse:");
    println!("{}", response.content);

    Ok(())
}
