use std::env;
use tokio::io::{stdout, AsyncWriteExt};

use nl_llm::client::LlmClient;
use nl_llm::primitive::PrimitiveRequest;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    stdout().flush().await?;

    let api_key =
        env::var("VERCEL_AI_GATEWAY_API_KEY").unwrap_or_else(|_| "your_api_key_here".to_string());

    let client = LlmClient::from_preset("vercel_ai_gateway")
        .expect("Vercel AI Gateway preset not found")
        .with_api_key(&api_key)
        .build();

    println!("🚀 [Vercel AI Gateway] 开始测试 Vercel AI Gateway 推理 API...");
    println!("📥 正在发送请求至 Vercel AI Gateway...");

    let request = PrimitiveRequest::single_user_message("用中文简单介绍一下你自己，不超过50个字")
        .with_model("openai/gpt-4o-mini"); // 推荐 provider/model 命名

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
