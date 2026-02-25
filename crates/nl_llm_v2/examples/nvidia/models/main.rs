//! Nvidia NIM 模型列表与别名

use nl_llm_v2::LlmClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("NVIDIA_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: nvidia_models <API_KEY>");
            eprintln!("或设置 NVIDIA_API_KEY 环境变量");
            std::process::exit(1);
        });

    let client = LlmClient::from_preset("nvidia")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    println!("========================================");
    println!("  Nvidia NIM 模型列表");
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

    println!("\n----------------------------------------");
    println!("  常用别名 (本地解析)");
    println!("----------------------------------------\n");

    let aliases = [
        ("nvidia / llama", "meta/llama-3.3-70b-instruct"),
        ("llama-405b", "meta/llama-3.1-405b-instruct"),
        ("nemotron", "nvidia/llama-3.1-nemotron-70b-instruct"),
        ("deepseek / r1", "deepseek-ai/deepseek-r1"),
        ("qwen", "qwen/qwen2.5-72b-instruct"),
        ("mistral", "mistralai/mistral-large-2-instruct"),
    ];
    for (alias, target) in aliases {
        println!("  '{}' -> '{}'", alias, target);
    }

    Ok(())
}
