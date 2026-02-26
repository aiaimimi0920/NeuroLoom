use anyhow::Result;
use nl_llm_v2::LlmClient;

#[tokio::main]
async fn main() -> Result<()> {
    let api_key = std::env::var("FASTGPT_API_KEY")
        .expect("FASTGPT_API_KEY 环境变量未设置");

    let client = LlmClient::from_preset("fastgpt")
        .expect("找不到 FastGPT 预设")
        .with_api_key(api_key)
        .build();

    println!("Fetching FastGPT models...\n");

    match client.list_models().await {
        Ok(models) => {
            println!("Found {} models:", models.len());
            for model in models {
                println!("- {} ({})", model.id, model.description);
            }
        }
        Err(e) => {
            eprintln!("Error fetching models: {}", e);
        }
    }

    Ok(())
}
