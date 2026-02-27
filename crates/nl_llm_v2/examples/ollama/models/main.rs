use anyhow::Result; use nl_llm_v2::LlmClient;
#[tokio::main] async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let api_key = std::env::var("OLLAMA_API_KEY").unwrap_or_else(|_| args.get(1).cloned().unwrap_or_default());
    println!("=== Ollama Models ===");
    let client = LlmClient::from_preset("ollama").expect("preset").with_ollama_auth(&api_key).build();
    match client.list_models().await {
        Ok(models) => {
            for (i, m) in models.iter().enumerate() { println!("{}. {}", i + 1, m.id); }
            println!("\nTotal models: {}", models.len());
        },
        Err(e) => println!("Error listing models: {}", e)
    }
    Ok(())
}
