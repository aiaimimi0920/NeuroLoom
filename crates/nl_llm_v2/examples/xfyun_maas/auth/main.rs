use anyhow::Result;
use nl_llm_v2::{LlmClient, PrimitiveRequest};
#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let api_key = std::env::var("XFYUN_MAAS_API_KEY").ok().or_else(|| args.get(1).cloned()).unwrap_or_else(|| "dummy".to_string());
    let model = std::env::var("XFYUN_MAAS_MODEL").ok().or_else(|| args.get(2).cloned()).unwrap_or_else(|| "xopglm5".to_string());
    println!("=== MaaS Auth Test ===\n  Model: {}\n", model);
    let client = LlmClient::from_preset("xfyun_maas").expect("preset").with_xfyun_maas_auth(&api_key).build();
    let req = PrimitiveRequest::single_user_message("hi").with_model(&model);
    match client.complete(&req).await {
        Ok(resp) => { println!("OK! Response: {}", resp.content); if let Some(u) = &resp.usage { println!("  Tokens: p={} c={} t={}", u.prompt_tokens, u.completion_tokens, u.total_tokens); } }
        Err(e) => { println!("FAIL: {}", e); }
    }
    Ok(())
}
