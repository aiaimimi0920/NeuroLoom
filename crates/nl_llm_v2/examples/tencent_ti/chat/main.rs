use nl_llm_v2::auth::providers::tencent_ti::TencentTiAuth;
use nl_llm_v2::client::LlmClient;
use nl_llm_v2::primitive::PrimitiveRequest;
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Read secret ID and secret key from environment variables or text files
    let secret_id = env::var("TENCENT_SECRET_ID").expect("TENCENT_SECRET_ID must be set");
    let secret_key = env::var("TENCENT_SECRET_KEY").expect("TENCENT_SECRET_KEY must be set");

    // Initialize the Tencent Ti Auth Provider
    // The action for standard chat completions in Tencent TI is typically "ChatCompletions"
    let auth = TencentTiAuth::new(
        secret_id,
        secret_key,
        "ChatCompletions",
        "hunyuan.tencentcloudapi.com",
        "hunyuan",
        "2023-09-01",
        "", // generic region or specific if needed
    );

    // Build the client using the pre-configured tencent_ti preset
    let client = LlmClient::from_preset("tencent_ti")
        .ok_or_else(|| anyhow::anyhow!("Preset not found"))?
        .auth(auth)
        .build();

    let request = PrimitiveRequest::single_user_message("请介绍一下腾讯混元 T1 模型的特点，并用幽默的风格讲一个程序员的笑话。")
        .with_model("hunyuan-t1-latest");

    println!("Sending chat request to Tencent TI (hunyuan-t1-latest)...");
    match client.complete(&request).await {
        Ok(response) => {
            println!("\nChat Response:");
            println!("{:#?}", response);
        }
        Err(e) => {
            println!("Request failed: {:?}", e);
        }
    }

    Ok(())
}
