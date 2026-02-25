//! 讯飞星火 流式输出
//!
//! 运行方式: cargo run -p nl_llm_v2 --example spark_stream -- <api_password|api_key:api_secret> [prompt]

use futures::StreamExt;
use nl_llm_v2::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("SPARK_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("需要提供 SPARK_API_KEY");
            std::process::exit(1);
        });

    let prompt = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "写一首关于人工智能的五言绝句。".to_string());

    println!("========================================");
    println!("  讯飞星火 流式输出");
    println!("========================================\n");

    let client = LlmClient::from_preset("spark_x")
        .expect("Preset should exist")
        .with_spark_auth(&api_key)
        .build();

    println!("模型: {}", client.resolve_model("ultra"));
    println!("用户: {}\n", prompt);
    print!("AI: ");

    let req = PrimitiveRequest::single_user_message(&prompt);

    let mut stream = client.stream(&req).await?;
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(c) => print!("{}", c.content),
            Err(e) => {
                eprintln!("\n流式错误: {}", e);
                break;
            }
        }
    }
    println!("\n");

    Ok(())
}
