use nl_llm::{primitive::PrimitiveRequest, LlmClient};

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

    let request = PrimitiveRequest::single_user_message("302.ai，你好！请问你是谁？");
    println!("发送请求...");
    let response = client.complete(&request).await?;
    println!("响应内容:\n\n{}", response.content);

    Ok(())
}
