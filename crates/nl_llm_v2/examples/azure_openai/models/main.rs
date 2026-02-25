//! Azure OpenAI 模型列表

use nl_llm_v2::LlmClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("========================================");
    println!("  Azure OpenAI 可部署模型");
    println!("========================================\n");

    let client = LlmClient::from_preset("azure_openai")
        .expect("Preset should exist")
        .with_api_key("placeholder")
        .build();

    match client.list_models().await {
        Ok(models) => {
            println!("Azure 上常见的可部署模型 (共 {} 个):\n", models.len());
            for (i, m) in models.iter().enumerate() {
                println!("  {}. {} — {}", i + 1, m.id, m.description);
            }
        }
        Err(e) => { println!("获取失败: {}", e); }
    }

    println!("\n----------------------------------------");
    println!("  使用说明");
    println!("----------------------------------------\n");
    println!("  Azure OpenAI 需要你先在 Azure Portal 中部署模型。");
    println!("  \"模型名\" 实际上就是你的 deployment 名称。\n");
    println!("  配置 examples/.env.local:");
    println!("    AZURE_OPENAI_ENDPOINT=https://your-resource.openai.azure.com");
    println!("    AZURE_OPENAI_KEY=your-api-key");
    println!("    AZURE_DEPLOYMENT=your-deployment-name");

    Ok(())
}
