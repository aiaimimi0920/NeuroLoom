use anyhow::Result;
use nl_llm_v2::presets;
use nl_llm_v2::provider::traits::LlmClient;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let api_key = std::env::var("JINA_API_KEY")
        .expect("JINA_API_KEY 环境变量未设置");

    let client = presets::REGISTRY
        .get_builder("jina")
        .expect("找不到 Jina 预设")
        .auth(nl_llm_v2::site::Auth::api_key(api_key))
        .build()?;

    println!("Fetching Jina models...\n");
    let models = client.list_models().await?;

    for model in models {
        println!("- {} (Provider: {})", model.id, model.provider);
        if let Some(desc) = model.description {
            println!("  Description: {}", desc);
        }
        println!("  Capabilities: {:?}", model.capabilities);
        println!("  Context length: {}", model.context_length);
        println!();
    }

    Ok(())
}
