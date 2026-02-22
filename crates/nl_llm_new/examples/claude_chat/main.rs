//! Claude 对话测试

use nl_llm_new::primitive::PrimitiveRequest;
use nl_llm_new::provider::claude::config::ClaudeConfig;
use nl_llm_new::provider::claude::provider::ClaudeProvider;
use nl_llm_new::provider::LlmProvider;
use futures::StreamExt;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let use_stream = args.iter().any(|a| a == "--stream");

    let api_key = args.windows(2).find(|w| w[0] == "--key").map(|w| w[1].clone())
        .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
        .unwrap_or_else(|| {
            eprintln!("请提供 API Key: --key sk-ant-... 或设置 ANTHROPIC_API_KEY");
            std::process::exit(1);
        });

    let model = args.windows(2).find(|w| w[0] == "--model").map(|w| w[1].clone())
        .unwrap_or_else(|| "claude-sonnet-4-20250514".to_string());

    let prompt = "Hello! Please introduce yourself briefly.".to_string();

    println!("========================================");
    println!("  Claude Chat (nl_llm_new)");
    println!("========================================");
    println!("  Key: {}...", &api_key[..12.min(api_key.len())]);
    println!("  Model: {}", model);
    println!("========================================");
    println!();

    let provider = ClaudeProvider::new(ClaudeConfig {
        api_key,
        model,
        base_url: None,
        max_tokens: Some(4096),
    });

    let primitive = PrimitiveRequest::single_user_message(&prompt);
    let body = provider.compile(&primitive);

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        if use_stream {
            match provider.stream(body).await {
                Ok(mut stream) => {
                    print!("AI: ");
                    while let Some(chunk) = stream.next().await {
                        match chunk {
                            Ok(c) => {
                                if let nl_llm_new::provider::ChunkDelta::Text(t) = &c.delta {
                                    print!("{}", t);
                                }
                            }
                            Err(e) => { eprintln!("\nError: {:?}", e); break; }
                        }
                    }
                    println!();
                }
                Err(e) => eprintln!("Error: {:?}", e),
            }
        } else {
            match provider.complete(body).await {
                Ok(response) => println!("AI: {}", response.content),
                Err(e) => { eprintln!("Error: {:?}", e); std::process::exit(1); }
            }
        }
    });
}
