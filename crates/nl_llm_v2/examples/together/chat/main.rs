use tokio::io::{stdout, AsyncWriteExt};
use std::env;

use nl_llm_v2::client::LlmClient;
use nl_llm_v2::primitive::PrimitiveRequest;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    stdout().flush().await?;

    let api_key = env::var("TOGETHER_API_KEY").unwrap_or_else(|_| "your_api_key_here".to_string());

    let client = LlmClient::from_preset("together")
        .expect("Together preset not found")
        .with_api_key(&api_key)
        .build();

    println!("🚀 [Together] 开始测试 Together AI 推理...");
    println!("📥 正在发送请求至 Together (meta-llama/Llama-3.3-70B-Instruct-Turbo)...");

    let request = PrimitiveRequest::single_user_message("用中文简单介绍一下你自己，不超过50个字")
        .with_model("meta-llama/Llama-3.3-70B-Instruct-Turbo");

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
