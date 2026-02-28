use tokio::io::{stdout, AsyncWriteExt};
use std::env;

use nl_llm::client::LlmClient;
use nl_llm::primitive::PrimitiveRequest;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    stdout().flush().await?;

    let api_key = env::var("HUGGINGFACE_API_KEY").unwrap_or_else(|_| "your_api_key_here".to_string());

    let client = LlmClient::from_preset("huggingface")
        .expect("Hugging Face preset not found")
        .with_api_key(&api_key)
        .build();

    println!("🚀 [Hugging Face] 开始测试 Hugging Face 推理 API...");
    println!("📥 正在发送请求至 Hugging Face (meta-llama/Llama-3.3-70B-Instruct)...");

    let request = PrimitiveRequest::single_user_message("用中文简单介绍一下你自己，不超过50个字")
        .with_model("meta-llama/Llama-3.3-70B-Instruct");

    match client.complete(&request).await {
        Ok(response) => {
            println!("✅ 推理成功！");
            println!("📝 回复内容: {}", response.content);
        }
        Err(e) => {
            println!("❌ 请求失败: {}", e);
        }
    }

    Ok(())
}
