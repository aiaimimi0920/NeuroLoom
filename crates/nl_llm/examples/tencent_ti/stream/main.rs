use nl_llm::auth::providers::tencent_ti::TencentTiAuth;
use nl_llm::client::LlmClient;
use nl_llm::primitive::PrimitiveRequest;
use tokio_stream::StreamExt;
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Read secret ID and secret key from environment variables or text files
    let secret_id = env::var("TENCENT_SECRET_ID").expect("TENCENT_SECRET_ID must be set");
    let secret_key = env::var("TENCENT_SECRET_KEY").expect("TENCENT_SECRET_KEY must be set");

    // Initialize the Tencent Ti Auth Provider
    let auth = TencentTiAuth::new(
        secret_id,
        secret_key,
        "ChatCompletions",
        "hunyuan.tencentcloudapi.com",
        "hunyuan",
        "2023-09-01",
        "",
    );

    // Build the client using the pre-configured tencent_ti preset
    let client = LlmClient::from_preset("tencent_ti")
        .ok_or_else(|| anyhow::anyhow!("Preset not found"))?
        .auth(auth)
        .build();

    let request = PrimitiveRequest::single_user_message("用Rust语言写一个并发的Hello World，并详细注释。")
        .with_model("hunyuan-lite");

    println!("Sending streaming request to Tencent TI (hunyuan-lite)...");
    match client.stream(&request).await {
        Ok(mut stream) => {
            println!("\nStreaming Response:");
            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        print!("{}", chunk.content);
                    }
                    Err(e) => {
                        println!("\nError in stream chunk: {}", e);
                        break;
                    }
                }
            }
            println!();
        }
        Err(e) => {
            println!("Stream request failed: {:?}", e);
        }
    }

    Ok(())
}
