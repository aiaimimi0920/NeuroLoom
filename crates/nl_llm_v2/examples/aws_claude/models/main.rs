//! AWS Claude (Amazon Bedrock) 模型列表

use nl_llm_v2::LlmClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("========================================");
    println!("  AWS Claude (Bedrock) 模型列表");
    println!("========================================\n");

    let client = LlmClient::from_preset("aws_claude")
        .expect("Preset should exist")
        .with_api_key("placeholder")
        .build();

    match client.list_models().await {
        Ok(models) => {
            println!("Bedrock 上可用的 Claude 模型 (共 {} 个):\n", models.len());
            for (i, m) in models.iter().enumerate() {
                println!("  {}. {}\n     {}\n", i + 1, m.id, m.description);
            }
        }
        Err(e) => { println!("获取失败: {}", e); }
    }

    println!("----------------------------------------");
    println!("  常用别名");
    println!("----------------------------------------\n");
    let aliases = [
        ("aws / sonnet / claude", "anthropic.claude-sonnet-4-6-20250514-v1:0"),
        ("opus", "anthropic.claude-opus-4-6-20250514-v1:0"),
        ("sonnet-4.5", "anthropic.claude-sonnet-4-5-20250929-v1:0"),
        ("3.5", "anthropic.claude-3-5-sonnet-20241022-v2:0"),
        ("haiku", "anthropic.claude-3-5-haiku-20241022-v1:0"),
    ];
    for (a, t) in aliases { println!("  '{}' -> '{}'", a, t); }

    println!("\n----------------------------------------");
    println!("  认证配置说明");
    println!("----------------------------------------\n");
    println!("  AK/SK 模式:");
    println!("    AWS_ACCESS_KEY_ID=AKIA...");
    println!("    AWS_SECRET_ACCESS_KEY=xxxxx");
    println!("    AWS_REGION=us-east-1\n");
    println!("  API Key 模式:");
    println!("    AWS_BEDROCK_API_KEY=xxx");

    Ok(())
}
