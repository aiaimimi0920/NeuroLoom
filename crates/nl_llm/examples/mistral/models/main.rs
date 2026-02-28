use anyhow::Result;
use nl_llm::LlmClient;

#[tokio::main]
async fn main() -> Result<()> {
    let api_key = std::env::var("MISTRAL_API_KEY").expect("MISTRAL_API_KEY 环境变量未设置");

    let client = LlmClient::from_preset("mistral")
        .expect("找不到 Mistral 预设")
        .with_api_key(api_key)
        .build();

    println!("Fetching models permitted under Mistral AI key...\n");

    match client.list_models().await {
        Ok(models) => {
            println!("Found {} models:", models.len());
            for model in models {
                println!("- {}", model.id);
                // The underlying structures might hold generic model information
                // We just want to make sure the HTTP call worked and returned an array.
            }
        }
        Err(e) => {
            eprintln!("Failed to parse models. This specific site endpoint (`/v1/models`) may not be configured to return valid response arrays if it overrides `list_models` logic: {:?}", e);
        }
    }

    Ok(())
}
