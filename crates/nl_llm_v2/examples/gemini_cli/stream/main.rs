//! gemini_cli 平台测试 - stream
//!
//! 运行方式: cargo run --example gemini_cli_stream
//! 或直接运行: test.bat

use nl_llm_v2::{LlmClient, PrimitiveRequest};
use anyhow::Result;
use std::path::PathBuf;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let cache_path: PathBuf = std::env::var("GEMINI_CLI_CACHE_PATH").ok()
        .or_else(|| args.get(1).cloned())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            path.push("examples");
            path.push("gemini_cli");
            path.push(".cache");
            path.push("oauth_token.json");
            path
        });

    println!("缓存文件: {}\n", cache_path.display());

    let client = LlmClient::from_preset("gemini_cli")
        .expect("Preset should exist")
        .with_gemini_cli_oauth(&cache_path)
        .build();

    let prompt = args.get(2).cloned()
        .unwrap_or_else(|| "Hello! Tell me a long story.".to_string());

    let req = PrimitiveRequest::single_user_message(&prompt)
        .with_model("gemini-2.5-flash");

    println!("用户: {}\n", prompt);
    println!("AI:");

    let mut stream = client.stream(&req).await?;
    
    use std::io::Write;
    while let Some(chunk_res) = stream.next().await {
        match chunk_res {
            Ok(chunk) => {
                print!("{}", chunk.content);
                std::io::stdout().flush().unwrap();
            }
            Err(e) => {
                eprintln!("\n流读取中断: {}", e);
                break;
            }
        }
    }
    println!("\n\n[流输出结束]");

    Ok(())
}
