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

    let request = PrimitiveRequest::new("302.ai，你好！请问你是谁？");
    println!("发送请求...");
    let response = client.generate(&request).await?;
    println!("响应内容:\n\n{}", response.content());
    
    Ok(())
}
