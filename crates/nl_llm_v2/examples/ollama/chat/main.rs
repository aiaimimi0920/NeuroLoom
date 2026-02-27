use anyhow::Result; use nl_llm_v2::{LlmClient, PrimitiveRequest};
#[tokio::main] async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let api_key = std::env::var("OLLAMA_API_KEY").unwrap_or_else(|_| args.get(1).cloned().unwrap_or_default());
    let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| args.get(2).cloned().unwrap_or_else(|| "llama3".to_string()));
    let prompt = args.get(3).cloned().unwrap_or_else(|| "hi".to_string());
    println!("=== Ollama Chat ==="); println!("Model: {}\nPrompt: {}", model, prompt);
    let client = LlmClient::from_preset("ollama").expect("preset").with_ollama_auth(&api_key).build();
    let req = PrimitiveRequest::single_user_message(&prompt).with_model(&model);
    match client.complete(&req).await {
        Ok(res) => {
            println!("\nAI: {}", res.content);
            if let Some(u) = &res.usage { println!("[Usage] prompt={} completion={} total={}", u.prompt_tokens, u.completion_tokens, u.total_tokens); }
        }
        Err(e) => println!("Error (is Ollama running?): {}", e)
    }
    Ok(())
}
