use anyhow::Result;
use nl_llm_v2::presets;
use nl_llm_v2::provider::traits::LlmClient;

#[tokio::main]
async fn main() -> Result<()> {
    // 默认开启控制台日志
    tracing_subscriber::fmt::init();

    let api_key = std::env::var("DIFY_API_KEY")
        .expect("DIFY_API_KEY 环境变量未设置");

    let client = presets::REGISTRY
        .get_builder("dify")
        .expect("找不到 Dify 预设")
        .auth(nl_llm_v2::site::Auth::api_key(api_key))
        .build()?;

    println!("Sending message to Dify...");
    let response = client.complete("dify", "你好，请用一句话介绍自己").await?;

    println!("\nResponse:");
    println!("{}", response.content);

    Ok(())
}
