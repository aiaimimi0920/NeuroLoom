//! Vertex AI (SA JSON) 对话测试

use nl_llm_new::primitive::PrimitiveRequest;
use nl_llm_new::provider::vertex::VertexProvider;
use nl_llm_new::provider::LlmProvider;
use futures::StreamExt;
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let use_stream = args.iter().any(|a| a == "--stream");

    // SA JSON 路径
    let sa_path = args.windows(2).find(|w| w[0] == "--sa").map(|w| w[1].clone())
        .or_else(|| std::env::var("VERTEX_SA_JSON_PATH").ok())
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("examples")
                .join("vertex")
                .join("vertex_sa.json")
                .to_string_lossy()
                .to_string()
        });

    let model = args.windows(2).find(|w| w[0] == "--model").map(|w| w[1].clone())
        .or_else(|| std::env::var("VERTEX_MODEL").ok())
        .unwrap_or_else(|| "gemini-2.5-flash".to_string());

    let location = args.windows(2).find(|w| w[0] == "--location").map(|w| w[1].clone())
        .or_else(|| std::env::var("VERTEX_LOCATION").ok());

    let prompt = args.iter().skip(1)
        .filter(|a| !a.starts_with("--") && {
            let idx = args.iter().position(|x| x == *a).unwrap();
            idx == 0 || (args[idx-1] != "--sa" && args[idx-1] != "--model" && args[idx-1] != "--location")
        })
        .cloned()
        .next()
        .unwrap_or_else(|| "Hello! Please introduce yourself briefly.".to_string());

    println!("========================================");
    println!("  Vertex AI Chat (nl_llm_new)");
    println!("========================================");
    println!("  SA JSON: {}", sa_path);
    println!("  Model: {}", model);
    println!("  Mode: {}", if use_stream { "stream" } else { "non-stream" });
    println!("========================================");
    println!();
    println!("User: {}", prompt);
    println!();

    let sa_json = match std::fs::read_to_string(&sa_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to read SA JSON: {}", e);
            std::process::exit(1);
        }
    };

    let provider = VertexProvider::from_service_account(sa_json, model, location);

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
                            Err(e) => { eprintln!("\nStream error: {:?}", e); break; }
                        }
                    }
                    println!();
                }
                Err(e) => eprintln!("Error: {:?}", e),
            }
        } else {
            match provider.complete(body).await {
                Ok(response) => {
                    println!("AI: {}", response.content);
                }
                Err(e) => { eprintln!("Error: {:?}", e); std::process::exit(1); }
            }
        }
    });
}
