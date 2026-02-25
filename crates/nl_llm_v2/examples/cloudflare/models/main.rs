//! Cloudflare Workers AI 模型列表

use nl_llm_v2::LlmClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("========================================");
    println!("  Cloudflare Workers AI 模型列表");
    println!("========================================\n");

    let client = LlmClient::from_preset("cloudflare")
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
        ("cloudflare / llama / llama-3.3", "@cf/meta/llama-3.3-70b-instruct-fp8-fast"),
        ("llama-70b", "@cf/meta/llama-3.1-70b-instruct"),
        ("llama-8b", "@cf/meta/llama-3.1-8b-instruct"),
        ("deepseek / r1", "@cf/deepseek-ai/deepseek-r1-distill-qwen-32b"),
        ("mistral", "@cf/mistral/mistral-7b-instruct-v0.2-lora"),
        ("qwen", "@cf/qwen/qwen1.5-14b-chat-awq"),
        ("gemma", "@hf/google/gemma-7b-it"),
    ];
    for (a, t) in aliases { println!("  '{}' -> '{}'", a, t); }

    println!("\n----------------------------------------");
    println!("  认证配置说明");
    println!("----------------------------------------\n");
    println!("  环境变量: CLOUDFLARE_API_TOKEN=xxx");
    println!("\n  获取凭据:");
    println!("    1. Account ID: Cloudflare Dashboard 右侧");
    println!("    2. API Token: https://dash.cloudflare.com/profile/api-tokens");
    println!("\n  免费额度:");
    println!("    - 每日 10,000 神经元免费");
    println!("    - Llama 3.1 8B 几乎免费可用");

    Ok(())
}
