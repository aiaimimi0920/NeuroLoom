//! Azure OpenAI 流式输出
//!
//! 运行方式:
//!   set AZURE_OPENAI_API_KEY=your-key
//!   set AZURE_OPENAI_ENDPOINT=https://YOUR-RESOURCE.openai.azure.com
//!   cargo run -p nl_llm_v2 --example azure_openai_stream

use futures::StreamExt;
use nl_llm_v2::{LlmClient, PrimitiveRequest};
use std::io::Write;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("AZURE_OPENAI_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: azure_openai_stream <API_KEY> <ENDPOINT> <DEPLOYMENT>");
            eprintln!("或设置环境变量:");
            eprintln!("  AZURE_OPENAI_API_KEY");
            eprintln!("  AZURE_OPENAI_ENDPOINT");
            std::process::exit(1);
        });

    let endpoint = std::env::var("AZURE_OPENAI_ENDPOINT")
        .or_else(|_| std::env::args().nth(2).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("错误: 需要设置 AZURE_OPENAI_ENDPOINT 环境变量");
            eprintln!("示例: https://YOUR-RESOURCE.openai.azure.com");
            std::process::exit(1);
        });

    let deployment = std::env::args().nth(3).unwrap_or_else(|| "gpt-4o".to_string());

    let client = LlmClient::from_preset("azure_openai")
        .expect("Preset should exist")
        .with_base_url(&endpoint)
        .with_api_key(&api_key)
        .build();

    println!("========================================");
    println!("  Azure OpenAI 流式输出");
    println!("========================================\n");
    println!("Endpoint: {}", endpoint);
    println!("Deployment: {}", deployment);
    print!("\nAI: ");
    std::io::stdout().flush()?;

    let req = PrimitiveRequest::single_user_message("写一段简单的快速排序代码")
        .with_model(&deployment);

    let mut stream = client.stream(&req).await?;
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(c) => { print!("{}", c.content); std::io::stdout().flush()?; }
            Err(e) => { eprintln!("\n读取流错误: {}", e); break; }
        }
    }
    println!("\n");
    Ok(())
}
