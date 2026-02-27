use futures::StreamExt;
use nl_llm_v2::client::LlmClient;
use nl_llm_v2::primitive::PrimitiveRequest;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    
    // 从环境变量中读取端点和密钥
    let base_url = std::env::var("BURNCLOUD_BASE_URL").unwrap_or_else(|_| "https://api.burn.hair/v1".to_string());
    
    // BurnCloud 测试可以接收真实的，或者是用于测试的 dummy 伪造密钥
    let api_key = std::env::var("NL_API_KEY")
        .unwrap_or_else(|_| "sk-dummy-test-key-for-burncloud".to_string());
    
    let client = LlmClient::build_burncloud_with_url(&base_url, &api_key);

    let mut req = PrimitiveRequest::single_user_message("请用一句话解释什么是 Rust 的所有权机制？")
        .with_model("gpt-4o");
    req.system = Some("你是一个专业的编程助手，请用中文简短回答。".to_string());

    println!("=================== 开始流式请求 (BurnCloud) ===================");
    let mut stream = client.stream(&req).await?;

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(c) => print!("{}", c.content),
            Err(e) => {
                // 白盒测试期望这里捕获并正确处理 HTTP 错误（如未授权、无额度等）
                eprintln!("\n捕捉到错误 (可预期的白盒结果): {:?}", e);
            }
        }
    }
    println!("\n=================== 结束 ===================");

    Ok(())
}
