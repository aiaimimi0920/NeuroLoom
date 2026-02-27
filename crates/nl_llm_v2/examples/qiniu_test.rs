use dotenv::dotenv;
use futures::StreamExt;
use nl_llm_v2::client::LlmClient;
use nl_llm_v2::primitive::PrimitiveRequest;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    
    // 从环境变量中读取密钥
    let api_key = std::env::var("NL_API_KEY").expect("请设置 NL_API_KEY 环境变量");
    
    // 初始化 Qiniu 客户端实例
    let client = LlmClient::build_qiniu(&api_key);

    let mut req = PrimitiveRequest::chat("qwen-plus");
    req.add_system("你是一个专业的编程助手，请用中文简短回答。");
    req.add_user("请用一句话解释什么是 Rust 的所有权机制？");

    println!("=================== 开始流式请求 (Qiniu AI) ===================");
    let mut stream = client.generate_stream(req).await?;

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(c) => print!("{}", c.content),
            Err(e) => eprintln!("\nError: {:?}", e),
        }
    }
    println!("\n=================== 结束 ===================");

    Ok(())
}
