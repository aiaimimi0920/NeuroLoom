use anyhow::Result;
use nl_llm::{LlmClient, PrimitiveRequest};
#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let api_key = std::env::var("XFYUN_MAAS_API_KEY").ok().or_else(|| args.get(1).cloned()).unwrap_or_else(|| "dummy".to_string());
    let model = std::env::var("XFYUN_MAAS_MODEL").ok().or_else(|| args.get(2).cloned()).unwrap_or_else(|| "xopglm5".to_string());
    let prompt = args.get(3).cloned().unwrap_or_else(|| "hi".to_string());
    let client = LlmClient::from_preset("xfyun_maas").expect("preset").with_xfyun_maas_auth(api_key).build();
    let mut req = PrimitiveRequest::single_user_message(&prompt).with_model(&model);
    req.stream = true;
    println!("Model: {}\nUser: {}\n\nAI (Stream):", model, prompt);
    let mut stream = client.stream(&req).await?;
    use tokio_stream::StreamExt;
    while let Some(chunk) = stream.next().await { if let Ok(c) = chunk { print!("{}", c.content); } }
    println!();
    Ok(())
}
