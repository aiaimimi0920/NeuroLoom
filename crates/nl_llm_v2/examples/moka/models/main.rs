use anyhow::Result;
use nl_llm_v2::LlmClient;

#[tokio::main]
async fn main() -> Result<()> {
    let api_key = std::env::var("MOKA_API_KEY")
        .expect("MOKA_API_KEY 环境变量未设置");

    // Initialize the MokaAI preset client
    let client = LlmClient::from_preset("moka")
        .expect("找不到 moka 预设")
        .with_api_key(api_key)
        .build();

    println!("Fetching MokaAI associated models...\n");
    
    // Testing the generic /models OpenAi extension route mapping
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
