//! qiniu 平台测试 - stream
//!
//! 运行方式: cargo run --example qiniu_stream
//! 或直接运行: test.bat

use anyhow::Result;
use nl_llm::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let api_key = std::env::var("QINIU_API_KEY")
        .ok()
        .or_else(|| std::env::var("NL_API_KEY").ok())
        .or_else(|| args.get(1).cloned())
        .unwrap_or_else(|| "dummy_credential".to_string());

    let prompt = args
        .get(2)
        .cloned()
        .unwrap_or_else(|| "请用三点总结 Rust 的内存安全优势。".to_string());

    let client = LlmClient::build_qiniu(api_key);

    let mut req = PrimitiveRequest::single_user_message(&prompt).with_model("qwen-plus");
    req.stream = true;

    println!("用户: {}\n", prompt);
    println!("AI(流式):");

    let mut stream = client.stream(&req).await?;
    use tokio_stream::StreamExt;
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(c) => print!("{}", c.content),
            Err(e) => eprintln!("\nError: {:?}", e),
        }
    }
    println!();

    Ok(())
}
