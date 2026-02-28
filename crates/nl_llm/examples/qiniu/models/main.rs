//! qiniu 平台测试 - models
//!
//! 运行方式: cargo run --example qiniu_models
//! 或直接运行: test.bat

use anyhow::Result;
use nl_llm::LlmClient;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let api_key = std::env::var("QINIU_API_KEY")
        .ok()
        .or_else(|| std::env::var("NL_API_KEY").ok())
        .or_else(|| args.get(1).cloned())
        .unwrap_or_else(|| "dummy_credential".to_string());

    let client = LlmClient::build_qiniu(api_key);
    let models = client.list_models().await?;

    println!("可用模型数量: {}", models.len());
    for model in models.iter().take(20) {
        println!("- {}", model.id);
    }

    Ok(())
}
