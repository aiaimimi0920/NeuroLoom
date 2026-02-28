use nl_llm::client::LlmClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 设置日志
    std::env::set_var("RUST_LOG", "debug");

    // 1. 初始化 Coze 客户端并注入 API Key
    let api_key = std::env::var("COZE_API_KEY").expect("请设置 COZE_API_KEY 环境变量");
    let client = LlmClient::from_preset("coze")
        .expect("Coze preset not found")
        .with_api_key(api_key)
        .build();

    // 2. 调用 Extension 接口获取可用模型列表
    println!("\n▶ 正在从 Coze API 查询可用模型 (Bots) ...");

    match client.list_models().await {
        Ok(models) => {
            println!("\n✅ Coze Extension 模型列表 (实际您需要使用 Bot ID 替代):");
            for m in models {
                println!("  - [{}] {}", m.id, m.description);
            }
        }
        Err(e) => {
            println!("\n❌ 获取模型列表失败: {}", e);
        }
    }

    Ok(())
}
