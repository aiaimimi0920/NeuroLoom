use anyhow::Result;
use nl_llm_v2::{LlmClient, PrimitiveRequest};

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
        .unwrap_or_else(|| "请简单介绍一下你自己。".to_string());

    println!("=== LM Studio Chat ===");
    println!("Model: {}\nPrompt: {}", model, prompt);

    let client = LlmClient::from_preset("lmstudio")
        .expect("lmstudio preset")
        .with_ollama_auth(&api_key)
        .build();

    let req = PrimitiveRequest::single_user_message(&prompt).with_model(&model);

    match client.complete(&req).await {
        Ok(resp) => {
            println!("\nAI: {}", resp.content);
            if let Some(usage) = &resp.usage {
                println!(
                    "[Usage] prompt={} completion={} total={}",
                    usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
                );
            }
        }
        Err(err) => {
            eprintln!("Error (is LM Studio server running at 127.0.0.1:1234?): {err}");
        }
    }

    Ok(())
}
