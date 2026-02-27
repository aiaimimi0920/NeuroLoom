use futures_util::StreamExt;
use nl_llm_v2::{
    LlmClient, primitive::PrimitiveRequest,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 设置日志
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    // 可以在环境变量中设置或者在这里显式设置 302.ai API KEY
    let api_key = std::env::var("A302_API_KEY")
        .unwrap_or_else(|_| "sk-lRQzthyTyLZ5zoREfLdi13xY4sbZWQjlgui7aFzB9D2hv38B".to_string());

    let client = LlmClient::from_preset("302.ai")
        .expect("Preset 302.ai not found")
        .with_api_key(&api_key)
        .build();

    let request = PrimitiveRequest::new("请给我讲一个关于人工智能和未来的科幻小故事，大概300字左右。");
    println!("发送请求...");
    
    let mut stream = client.generate_stream(&request).await?;
    
    println!("流式响应内容:");
    while let Some(chunk_res) = stream.next().await {
        match chunk_res {
            Ok(chunk) => {
                print!("{}", chunk.content);
                use std::io::Write;
                std::io::stdout().flush().unwrap();
            },
            Err(e) => {
                eprintln!("\n流读取错误: {}", e);
                break;
            }
        }
    }
    println!("\n\n完成!");
    
    Ok(())
}
