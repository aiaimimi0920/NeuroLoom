use nl_llm_v2::{LlmClient, Result};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 设置日志
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    // 302.ai 预设
    let api_key = std::env::var("A302_API_KEY")
        .unwrap_or_else(|_| "sk-lRQzthyTyLZ5zoREfLdi13xY4sbZWQjlgui7aFzB9D2hv38B".to_string());

    let client = LlmClient::from_preset("302.ai")
        .expect("Preset 302.ai not found")
        .with_api_key(&api_key)
        .build();

    println!("正在获取 302.ai 支持的模型列表...");
    
    match client.models().await {
        Ok(models) => {
            println!("成功获取到 {} 个模型:", models.len());
            for model in models {
                println!("- {}", model.id);
            }
        },
        Err(e) => {
            eprintln!("获取模型列表失败: {}", e);
        }
    }
    
    Ok(())
}
