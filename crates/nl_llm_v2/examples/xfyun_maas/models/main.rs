use anyhow::Result;
use nl_llm_v2::LlmClient;
#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let api_key = std::env::var("XFYUN_MAAS_API_KEY").ok().or_else(|| args.get(1).cloned()).unwrap_or_else(|| "dummy".to_string());
    let client = LlmClient::from_preset("xfyun_maas").expect("preset").with_xfyun_maas_auth(api_key).build();
    println!("=== MaaS Models ===");
    let models = client.list_models().await?;
    for (i, m) in models.iter().enumerate() { println!("  {}. {} - {}", i + 1, m.id, m.description); }
    println!("\nTotal: {} models", models.len());
    Ok(())
}
