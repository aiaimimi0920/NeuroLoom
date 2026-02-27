use anyhow::Result; use nl_llm_v2::{LlmClient, PrimitiveRequest}; use tokio_stream::StreamExt;
#[tokio::main] async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let api_key = std::env::var("OLLAMA_API_KEY").unwrap_or_else(|_| args.get(1).cloned().unwrap_or_default());
    let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| args.get(2).cloned().unwrap_or_else(|| "llama3".to_string()));
    let prompt = args.get(3).cloned().unwrap_or_else(|| "hi".to_string());
    println!("=== Ollama Stream ==="); println!("Model: {}\nPrompt: {}\nAI: ", model, prompt);
    let client = LlmClient::from_preset("ollama").expect("preset").with_ollama_auth(&api_key).build();
    let mut req = PrimitiveRequest::single_user_message(&prompt).with_model(&model);
    req.stream = true;
    match client.stream(&req).await {
        Ok(mut st) => while let Some(chunk) = st.next().await { if let Ok(c) = chunk { print!("{}", c.content); } },
        Err(e) => println!("Error (is Ollama running?): {}", e)
    }
    println!(); Ok(())
}
