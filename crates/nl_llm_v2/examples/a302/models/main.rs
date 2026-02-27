use nl_llm_v2::LlmClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    let api_key = std::env::var("A302_API_KEY").expect("请先设置环境变量 A302_API_KEY，再运行示例");

    let client = LlmClient::from_preset("302.ai")
        .expect("Preset 302.ai not found")
        .with_api_key(&api_key)
        .build();

    println!("正在获取 302.ai 支持的模型列表...");

    match client.list_models().await {
        Ok(models) => {
            println!("成功获取到 {} 个模型:", models.len());
            for model in models {
                println!("- {}", model.id);
            }
        }
        Err(e) => {
            eprintln!("获取模型列表失败: {}", e);
        }
    }

    Ok(())
}
