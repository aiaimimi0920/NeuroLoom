use anyhow::Result;
use nl_llm_v2::LlmClient;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let api_key = std::env::var("CUSTOM_API_KEY")
        .ok()
        .or_else(|| args.get(1).cloned())
        .unwrap_or_else(|| "sk-dummy-custom-key".to_string());

    let client = LlmClient::from_preset("custom")
        .expect("找不到 custom 预设")
        .with_api_key(api_key)
        .build();

    println!("Fetching Custom provider models...\n");
    let models = client.list_models().await?;

    for model in models {
        println!("- {}", model.id);
        if !model.description.is_empty() {
            println!("  Description: {}", model.description);
        }
    }

    Ok(())
}
