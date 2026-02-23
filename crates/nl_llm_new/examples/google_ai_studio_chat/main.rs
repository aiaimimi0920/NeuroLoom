//! Google AI Studio 对话测试

use nl_llm_new::primitive::PrimitiveRequest;
use nl_llm_new::provider::gemini::GeminiProvider;
use nl_llm_new::provider::LlmProvider;
use futures::StreamExt;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let use_stream = args.iter().any(|a| a == "--stream");

    let api_key = args.windows(2).find(|w| w[0] == "--key").map(|w| w[1].clone())
        .or_else(|| std::env::var("GOOGLE_AI_STUDIO_API_KEY").ok())
        .unwrap_or_else(|| {
            eprintln!("请提供 API Key: --key AIzaSy... 或设置 GOOGLE_AI_STUDIO_API_KEY");
            std::process::exit(1);
        });

    let model = args.windows(2).find(|w| w[0] == "--model").map(|w| w[1].clone())
        .unwrap_or_else(|| "gemini-2.5-flash".to_string());

    let prompt = "Hello! Please introduce yourself briefly.".to_string();

    println!("========================================");
    println!("  Google AI Studio Chat (nl_llm_new)");
    println!("========================================");
    println!("  Key: {}...", &api_key[..12.min(api_key.len())]);
    println!("  Model: {}", model);
    println!("========================================");
    println!();

    let provider = GeminiProvider::from_api_key(api_key, model);

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
