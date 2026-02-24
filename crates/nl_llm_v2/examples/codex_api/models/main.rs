//! Codex API 模型列表测试（API Key 模式）

use nl_llm_v2::LlmClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("OPENAI_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: codex_api_models <API_KEY>");
            std::process::exit(1);
        });

    println!("=== Codex API 模型列表 ===\n");

    let client = LlmClient::from_preset("codex_api")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    let models = client.list_models().await?;
    println!("可用模型 ({} 个):", models.len());
    for m in &models {
        println!("  - {} : {}", m.id, m.description);
    }

    Ok(())
}
