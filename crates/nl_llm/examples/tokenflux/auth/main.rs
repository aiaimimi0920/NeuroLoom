//! TokenFlux 平台测试 - auth
//!
//! 运行方式: cargo run --example tokenflux_auth
//! 或直接运行: test.bat

use anyhow::Result;
use nl_llm::LlmClient;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let api_key = std::env::var("TOKENFLUX_API_KEY")
        .ok()
        .or_else(|| std::env::var("NL_API_KEY").ok())
        .or_else(|| args.get(1).cloned())
        .unwrap_or_else(|| "dummy_credential".to_string());

    let client = LlmClient::from_preset("tokenflux")
        .expect("Preset not found")
        .with_api_key(api_key)
        .build();

    let models = client.list_models().await?;

    if models.is_empty() {
        println!("认证通过，但未返回模型列表");
    } else {
        println!("认证通过，模型列表示例: {}", models[0].id);
    }

    Ok(())
}
