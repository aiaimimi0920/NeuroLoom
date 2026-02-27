use futures::StreamExt;
use nl_llm_v2::{primitive::PrimitiveRequest, LlmClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    // 为避免误用泄露密钥，仅从环境变量读取
    let api_key = std::env::var("A302_API_KEY").expect("请先设置环境变量 A302_API_KEY，再运行示例");

    let client = LlmClient::from_preset("302.ai")
        .expect("Preset 302.ai not found")
        .with_api_key(&api_key)
        .build();

    let request =
        PrimitiveRequest::new("请给我讲一个关于人工智能和未来的科幻小故事，大概300字左右。");
    println!("发送请求...");

    let mut stream = client.stream(&request).await?;

    println!("流式响应内容:");
    while let Some(chunk_res) = stream.next().await {
        match chunk_res {
            Ok(chunk) => {
                print!("{}", chunk.content);
                use std::io::Write;
                std::io::stdout().flush().unwrap();
            }
            Err(e) => {
                eprintln!("\n流读取错误: {}", e);
                break;
            }
        }
    }
    println!("\n\n完成!");

    Ok(())
}
