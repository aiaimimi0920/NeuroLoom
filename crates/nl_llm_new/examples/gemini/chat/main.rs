//! Gemini Provider - 对话示例
//!
//! 支持官方端点和第三方代理站。
//!
//! 用法:
//!   cargo run --example gemini_chat -p nl_llm_new -- --key AIzaSy...
//!   cargo run --example gemini_chat -p nl_llm_new -- --key sk-xxx --base https://zenmux.ai/api
//!   cargo run --example gemini_chat -p nl_llm_new -- --key proxy-key --base http://127.0.0.1:3000 --stream

use nl_llm_new::primitive::PrimitiveRequest;
use nl_llm_new::provider::gemini::{GeminiConfig, GeminiProvider};
use nl_llm_new::auth::{ApiKeyConfig, ApiKeyProvider};
use nl_llm_new::provider::LlmProvider;
use futures::StreamExt;
use std::collections::HashMap;
use std::io::Write;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let use_stream = args.iter().any(|a| a == "--stream");

    let api_key = args.windows(2).find(|w| w[0] == "--key").map(|w| w[1].clone())
        .or_else(|| std::env::var("GEMINI_API_KEY").ok())
        .unwrap_or_else(|| {
            eprintln!("请提供 API Key: --key AIzaSy... 或设置 GEMINI_API_KEY 环境变量");
            std::process::exit(1);
        });

    let base_url = args.windows(2).find(|w| w[0] == "--base").map(|w| w[1].clone());

    let model = args.windows(2).find(|w| w[0] == "--model").map(|w| w[1].clone())
        .unwrap_or_else(|| "gemini-2.5-flash".to_string());

    let prompt = "你好！请用中文简单介绍一下你自己，以及你能做什么？".to_string();

    println!("========================================");
    println!("  Gemini API Chat (nl_llm_new)");
    println!("========================================");
    println!("  Mode: {}", if use_stream { "Stream" } else { "Complete" });
    if let Some(ref base) = base_url {
        println!("  Base URL: {}", base);
    } else {
        println!("  Base URL: Official Google AI Studio");
    }
    println!("  Model: {}", model);
    println!("========================================");
    println!();
    println!("用户: {}", prompt);
    println!();

    let provider = if let Some(base) = base_url {
        let mut auth_config = ApiKeyConfig::new(api_key, ApiKeyProvider::GeminiAIStudio);
        auth_config.base_url = Some(base);
        let mut config = GeminiConfig::new(auth_config, model.clone());
        
        let mut headers = HashMap::new();
        headers.insert("X-Router-Channel".to_string(), "forward-test".to_string());
        config.extra_headers = headers;
        
        GeminiProvider::new(config)
    } else {
        GeminiProvider::from_api_key(api_key, model.clone())
    };
    
    let mut primitive = PrimitiveRequest::single_user_message(&prompt);
    primitive.model = model;
    
    let body = provider.compile(&primitive);

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        if use_stream {
            match provider.stream(body).await {
                Ok(mut stream) => {
                    print!("AI: ");
                    std::io::stdout().flush().unwrap();
                    
                    while let Some(chunk) = stream.next().await {
                        match chunk {
                            Ok(c) => {
                                if let nl_llm_new::provider::ChunkDelta::Text(t) = &c.delta {
                                    print!("{}", t);
                                    std::io::stdout().flush().unwrap();
                                }
                            }
                            Err(e) => { 
                                eprintln!("\nError: {:?}", e); 
                                break; 
                            }
                        }
                    }
                    println!();
                }
                Err(e) => eprintln!("Error: {:?}", e),
            }
        } else {
            match provider.complete(body).await {
                Ok(response) => {
                    println!("AI:");
                    println!("{}", response.content);
                    println!("\n[Tokens - Input: {}, Output: {}]", 
                             response.usage.input_tokens, 
                             response.usage.output_tokens);
                },
                Err(e) => { 
                    eprintln!("Error: {:?}", e); 
                    std::process::exit(1); 
                }
            }
        }
    });
}
