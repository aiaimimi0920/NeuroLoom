use anyhow::Result;
use nl_llm_new::provider::gemini_cli::config::GeminiCliConfig;
use nl_llm_new::provider::gemini_cli::provider::GeminiCliProvider;
use nl_llm_new::primitive::{PrimitiveRequest, PrimitiveMessage};
use nl_llm_new::provider::traits::LlmProvider;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("..");
    path.push("nl_llm_v2");
    path.push("examples");
    path.push("gemini_cli");
    path.push(".cache");
    path.push("oauth_token.json");

    let config = GeminiCliConfig {
        token_path: path,
        model: "gemini-1.5-pro".to_string(),
    };
    
    let http = reqwest::Client::new();
    let provider = GeminiCliProvider::new(config, http)?;

    let mut req = PrimitiveRequest::default();
    req.model = "gemini-1.5-pro".to_string();
    req.messages.push(PrimitiveMessage::user("Hello"));

    println!("Sending via old provider...");

    let body = provider.compile(&req);
    println!(">>> OLD DBG PAYLOAD:\n{}", serde_json::to_string_pretty(&body).unwrap_or_default());

    match provider.complete(body).await {
        Ok(res) => println!("Success! {}", res.content),
        Err(e) => println!("Error! {:?}", e),
    }

    Ok(())
}
