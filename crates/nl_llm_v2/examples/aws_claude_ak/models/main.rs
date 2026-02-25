//! AWS Claude AK/SK 模式 — 模型列表

use nl_llm_v2::LlmClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("========================================");
    println!("  AWS Claude AK/SK 模式 — 模型列表");
    println!("========================================\n");

    let client = LlmClient::from_preset("aws_claude_ak")
        .expect("Preset should exist")
        .with_api_key("placeholder")
        .build();

    match client.list_models().await {
        Ok(models) => {
            println!("Bedrock Claude 模型 (共 {} 个):\n", models.len());
            for (i, m) in models.iter().enumerate() {
                println!("  {}. {}\n     {}\n", i + 1, m.id, m.description);
            }
        }
        Err(e) => { println!("获取失败: {}", e); }
    }

    println!("----------------------------------------");
    println!("  AK/SK 认证配置");
    println!("----------------------------------------\n");
    println!("  在 examples/.env.local 中配置:");
    println!("    AWS_ACCESS_KEY_ID=AKIA...");
    println!("    AWS_SECRET_ACCESS_KEY=xxxxx");
    println!("    AWS_REGION=us-east-1");
    println!("\n  URL 格式 (原生 Converse API):");
    println!("    https://bedrock-runtime.{{region}}.amazonaws.com/model/{{model-id}}/converse");

    Ok(())
}
