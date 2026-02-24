//! 智谱 BigModel (GLM国内版) 模型列表查询

use nl_llm_v2::LlmClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("ZHIPU_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| "placeholder".to_string());

    let client = LlmClient::from_preset("zhipu")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    println!("========================================");
    println!("  智谱 BigModel (GLM) 可用模型列表");
    println!("========================================\n");

    let models = client.list_models().await?;
    for (i, model) in models.iter().enumerate() {
        println!("  {}. {} — {}", i + 1, model.id, model.description);
    }
    println!("\n共 {} 个模型", models.len());

    Ok(())
}
