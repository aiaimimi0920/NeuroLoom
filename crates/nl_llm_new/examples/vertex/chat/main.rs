use anyhow::Result;
use nl_llm_new::primitive::{PrimitiveRequest, PrimitiveMetadata};
use nl_llm_new::provider::vertex::VertexProvider;
use nl_llm_new::provider::LlmProvider;
use std::env;
use std::io::{self, Write};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    // Ignore for now to avoid windows_sys dependency

    let args: Vec<String> = env::args().collect();
    let use_stream = args.iter().any(|a| a == "--stream");

    let prompt_idx = args.iter().position(|a| a == "--prompt");
    let prompt = if let Some(idx) = prompt_idx {
        if idx + 1 < args.len() {
            args[idx + 1].clone()
        } else {
            "你好！请用中文简单介绍一下你自己，以及你能做什么？".to_string()
        }
    } else {
        "你好！请用中文简单介绍一下你自己，以及你能做什么？".to_string()
    };

    let model = args.windows(2).find(|w| w[0] == "--model").map(|w| w[1].clone())
        .unwrap_or_else(|| "gemini-2.5-flash".to_string());

    let sa_json_path = args.windows(2).find(|w| w[0] == "--sa_json").map(|w| w[1].clone())
        .or_else(|| env::var("VERTEX_SA_JSON").ok());

    let location = args.windows(2).find(|w| w[0] == "--location").map(|w| w[1].clone())
        .unwrap_or_else(|| "us-central1".to_string());

    // 尝试直接读取作为文件路径，支持工作目录不同时的容错
    let mut sa_json_content = "{}".to_string();
    let cwd = std::env::current_dir().unwrap_or_default();
    
    // 候选路径列表
    let candidates = vec![
        std::path::PathBuf::from(&sa_json_path.clone().unwrap_or_default()),
        cwd.join(&sa_json_path.clone().unwrap_or_default()),
        cwd.join("crates/nl_llm_new").join(&sa_json_path.clone().unwrap_or_default()),
        std::path::PathBuf::from("../../").join(&sa_json_path.clone().unwrap_or_default()),
        std::path::PathBuf::from("../../crates/nl_llm_new").join(&sa_json_path.clone().unwrap_or_default()),
    ];

    let mut found_path = None;
    for path in &candidates {
        if path.is_file() {
            match std::fs::read_to_string(path) {
                Ok(content) => {
                    if content.trim().starts_with('{') {
                        sa_json_content = content;
                        found_path = Some(path.clone());
                        break;
                    }
                }
                Err(_) => continue,
            }
        }
    }

    let sa_json = if found_path.is_some() {
        sa_json_content
    } else if let Some(p) = &sa_json_path {
        if p.trim().starts_with('{') {
            p.to_string()
        } else {
            println!("警告: 未指定 VERTEX_SA_JSON 环境变量或提供 --sa_json 参数，将使用空配置。服务大概率会认证失败。");
            "{}".to_string()
        }
    } else {
        println!("警告: 未指定 VERTEX_SA_JSON 环境变量或提供 --sa_json 参数，将使用空配置。服务大概率会认证失败。");
        "{}".to_string()
    };

    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .expect("Failed to create HTTP client");

    let provider = VertexProvider::from_service_account(sa_json, model.clone(), Some(location), http);

    println!("========================================");
    println!("  Vertex API Chat (nl_llm_new)");
    println!("========================================");
    println!("  Mode: {}", if use_stream { "Stream" } else { "Complete" });
    println!("  Model: {}", model);
    println!("========================================\n");

    println!("用户: {}\n", prompt);
    println!("AI:");

    let mut req = PrimitiveRequest::single_user_message(&prompt);
    req.model = model;
    req.metadata = PrimitiveMetadata::default();
    
    let body = provider.compile(&req);

    if use_stream {
        let mut stream = provider.stream(body).await?;
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(c) => {
                    use nl_llm_new::provider::ChunkDelta;
                    match c.delta {
                        ChunkDelta::Text(text) => {
                            print!("{}", text);
                            io::stdout().flush()?;
                        }
                        ChunkDelta::ToolCall { .. } => {}
                        ChunkDelta::Thinking(text) => {
                            print!("[Thinking] {}", text);
                            io::stdout().flush()?;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("\n流式传输错误: {:?}", e);
                    break;
                }
            }
        }
        println!();
    } else {
        let resp = provider.complete(body).await?;
        println!("{}", resp.content);
        println!(
            "\n[Tokens - Input: {}, Output: {}]",
            resp.usage.input_tokens, resp.usage.output_tokens
        );
    }

    Ok(())
}

