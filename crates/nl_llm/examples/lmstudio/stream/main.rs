use anyhow::Result;
use nl_llm::{LlmClient, PrimitiveRequest};
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let api_key = std::env::var("LMSTUDIO_API_KEY")
        .unwrap_or_else(|_| args.get(1).cloned().unwrap_or_default());
    let model = std::env::var("LMSTUDIO_MODEL").unwrap_or_else(|_| {
        args.get(2)
            .cloned()
            .unwrap_or_else(|| "local-model".to_string())
    });
    let prompt = args
        .get(3)
        .cloned()
        .unwrap_or_else(|| "写一段简短的欢迎词。".to_string());

    println!("=== LM Studio Stream ===");
    println!("Model: {}\nPrompt: {}\nAI:", model, prompt);

    let client = LlmClient::from_preset("lmstudio")
        .expect("lmstudio preset")
        .with_ollama_auth(&api_key)
        .build();

    let mut req = PrimitiveRequest::single_user_message(&prompt).with_model(&model);
    req.stream = true;

    match client.stream(&req).await {
        Ok(mut stream) => {
            while let Some(chunk) = stream.next().await {
                if let Ok(c) = chunk {
                    print!("{}", c.content);
                }
            }
            println!();
        }
        Err(err) => {
            eprintln!("Error (is LM Studio server running at 127.0.0.1:1234?): {err}");
        }
    }

    Ok(())
}
