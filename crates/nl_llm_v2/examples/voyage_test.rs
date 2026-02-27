use dotenv::dotenv;
use nl_llm_v2::client::LlmClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    
    // 从环境变量中读取密钥
    let api_key = std::env::var("NL_API_KEY").expect("请设置 NL_API_KEY 环境变量");
    
    // 初始化 Voyage AI 客户端实例
    let client = LlmClient::build_voyage(&api_key);

    println!("=================== Voyage AI 配置验证 ===================");
    println!("提醒：Voyage AI 没有 /v1/chat/completions 服务，专门为 Embeddings 与 Reranking 设计。");
    
    // 获取支持的模型列表（这里调用了拓展扩展中的 list_models）
    println!("正在获取/展示适配模型列表...");
    let models = client.list_models().await?;
    
    for (i, model) in models.iter().enumerate() {
        println!("{}. {} - {}", i + 1, model.id, model.description);
    }
    
    println!("测试: voyage-code-2 的上下文限制被正确划定为: {}", client.max_context("voyage-code-2"));
    println!("测试: voyage-3 的上下文限制被正确划定为: {}", client.max_context("voyage-3"));
    
    println!("=================== 验证完成 ===================");

    Ok(())
}
