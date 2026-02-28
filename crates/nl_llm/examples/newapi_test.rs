use futures::StreamExt;
use nl_llm::client::LlmClient;
use nl_llm::primitive::PrimitiveRequest;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    
    // 从环境变量中读取端点和密钥
    let base_url = std::env::var("NEWAPI_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:3000/v1".to_string());
    let api_key = std::env::var("NL_API_KEY").expect("请设置 NL_API_KEY 环境变量");
    
    // 初始化 NewAPI 客户端实例
    let client = LlmClient::build_newapi(&base_url, &api_key);

    let mut req = PrimitiveRequest::single_user_message("请用一句话解释什么是 Rust 的所有权机制？")
        .with_model("gpt-4o");
    req.system = Some("你是一个专业的编程助手，请用中文简短回答。".to_string());

    println!("=================== 开始流式请求 ===================");
    let mut stream = client.stream(&req).await?;

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(c) => print!("{}", c.content),
            Err(e) => eprintln!("\nError: {:?}", e),
        }
    }
    println!("\n=================== 结束 ===================");

    Ok(())
}
