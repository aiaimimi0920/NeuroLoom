//! 智谱 BigModel (GLM国内版) 流式输出测试

use nl_llm_v2::{LlmClient, PrimitiveRequest};
use futures::StreamExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("ZHIPU_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: zhipu_stream <API_KEY> [prompt]");
            eprintln!("或设置 ZHIPU_API_KEY 环境变量");
            std::process::exit(1);
        });

    let prompt = std::env::args().nth(2)
        .unwrap_or_else(|| "用三句话介绍一下 Rust 语言。".to_string());

    let client = LlmClient::from_preset("zhipu")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    let req = PrimitiveRequest::single_user_message(&prompt)
        .with_model("glm-5");

    println!("用户: {}\n", prompt);
    print!("AI: ");

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
