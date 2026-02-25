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

    // Note: Jina might not have standard chat endpoints natively accessible like this if it's strictly embeddings/rerank.
    // However, if it supports standard Chat completions via some models, this will hit `/v1/chat/completions`.
    // We will use a fallback valid model or generic text if they support it.
    let model = "jina-embeddings-v3"; // Typically Jina is embeddings, but let's test a generic query if chat is supported or switch to embedding action natively if implemented later.
    
    println!("Sending message to Jina...");
    let response = client.complete(model, "Hello, Jina!").await?;

    println!("\nResponse:");
    println!("{}", response.content);

    Ok(())
}
