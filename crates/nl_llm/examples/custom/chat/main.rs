use anyhow::Result;
use nl_llm::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> Result<()> {
    // 允许通过环境变量或参数传入自定义测试键
    let args: Vec<String> = std::env::args().collect();
    let api_key = std::env::var("CUSTOM_API_KEY")
        .ok()
        .or_else(|| args.get(1).cloned())
        .unwrap_or_else(|| "sk-dummy-custom-key".to_string());

    let client = LlmClient::from_preset("custom")
        .expect("找不到 custom 预设")
        .with_api_key(api_key)
        .build();

    let default_prompt = "Hello, what is your custom model capabilities?".to_string();
    let prompt = args.get(2).cloned().unwrap_or(default_prompt);

    // 动态传入用户想要测试的任意模型名
    let model = args
        .get(3)
        .cloned()
        .unwrap_or_else(|| "gpt-3.5-turbo".to_string());

    let req = PrimitiveRequest::single_user_message(&prompt).with_model(&model);

    println!("User: {}", prompt);
    println!("Model Request: {}", model);
    println!("Waiting for Custom AI channel...\n");
    let resp = client.complete(&req).await?;

    println!("AI: {}", resp.content);

    Ok(())
}
