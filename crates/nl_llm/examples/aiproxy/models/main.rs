use anyhow::Result;
use nl_llm::LlmClient;

#[tokio::main]
async fn main() -> Result<()> {
    let api_key = std::env::var("AIPROXY_API_KEY").expect("AIPROXY_API_KEY 环境变量未设置");

    let client = LlmClient::from_preset("aiproxy")
        .expect("找不到 aiproxy 预设")
        .with_api_key(api_key)
        .build();

    println!("Fetching AI Proxy models...\n");

    let models = client.list_models().await?;

    for model in &models {
        println!("- {}", model.id);
        if !model.description.is_empty() {
            println!("  Description: {}", model.description);
        }
    }

    println!("\nTotal models fetched: {}", models.len());

    Ok(())
}
