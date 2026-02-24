//! BaiLing (百灵) 模型列表

use nl_llm_v2::LlmClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("BAILING_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: bailing_models <API_KEY>");
            eprintln!("或设置 BAILING_API_KEY 环境变量");
            std::process::exit(1);
        });

    let client = LlmClient::from_preset("bailing")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    println!("========================================");
    println!("  BaiLing (百灵) 模型列表");
    println!("========================================\n");

    match client.list_models().await {
        Ok(models) => {
            println!("共 {} 个模型:\n", models.len());
            for (i, m) in models.iter().enumerate() {
                println!("  {}. {} — {}", i + 1, m.id, m.description);
            }
        }
        Err(e) => { println!("❌ 获取失败: {}", e); std::process::exit(1); }
    }
    Ok(())
}
