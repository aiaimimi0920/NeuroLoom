use anyhow::Result; use nl_llm_v2::{LlmClient, PrimitiveRequest};
#[tokio::main] async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let api_key = args.get(1).cloned().unwrap_or_default();
    let model = args.get(2).cloned().unwrap_or_else(|| "xopglm5".to_string());
    let prompt = args.get(3).cloned().unwrap_or_else(|| "hi".to_string());
    let client = LlmClient::from_preset("xfyun_maas").expect("preset").auth(nl_llm_v2::auth::providers::xfyun_maas::XfyunMaasAuth::new(api_key)).build();
    let req = PrimitiveRequest::single_user_message(&prompt).with_model(&model);
    println!("Model: {}\nPrompt: {}\n", model, prompt);
    let resp = client.complete(&req).await?; println!("AI: {}", resp.content);
    if let Some(u) = &resp.usage { println!("[Usage] prompt={} completion={} total={}", u.prompt_tokens, u.completion_tokens, u.total_tokens); } Ok(())
}
