//! Perplexity AI 模型列表

use nl_llm_v2::LlmClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("========================================");
    println!("  Perplexity AI 模型列表");
    println!("========================================\n");

    let client = LlmClient::from_preset("perplexity")
        .expect("Preset should exist")
        .with_api_key("placeholder")
        .build();

    match client.list_models().await {
        Ok(models) => {
            println!("共 {} 个模型:\n", models.len());
            for (i, m) in models.iter().enumerate() {
                println!("  {}. {} — {}", i + 1, m.id, m.description);
            }
        }
        Err(e) => { println!("获取失败: {}", e); }
    }

    println!("\n----------------------------------------");
    println!("  常用别名");
    println!("----------------------------------------\n");
    let aliases = [
        ("perplexity / pplx", "sonar-pro"),
        ("sonar", "sonar"),
        ("reasoning", "sonar-reasoning-pro"),
        ("research", "sonar-deep-research"),
        ("r1", "r1-1776"),
    ];
    for (a, t) in aliases { println!("  '{}' -> '{}'", a, t); }

    println!("\n----------------------------------------");
    println!("  认证配置说明");
    println!("----------------------------------------\n");
    println!("  环境变量: PERPLEXITY_API_KEY=pplx-xxxx");
    println!("\n  获取密钥:");
    println!("    https://www.perplexity.ai → Settings → API");

    Ok(())
}
