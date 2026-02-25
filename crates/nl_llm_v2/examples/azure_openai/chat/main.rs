//! Azure OpenAI 基础对话

use nl_llm_v2::{LlmClient, PrimitiveRequest};
use nl_llm_v2::site::base::azure::AzureOpenAiSite;
use nl_llm_v2::protocol::base::openai::OpenAiProtocol;
use nl_llm_v2::model::azure_openai::AzureOpenAiModelResolver;
use nl_llm_v2::provider::azure_openai::AzureOpenAiExtension;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let endpoint = std::env::var("AZURE_OPENAI_ENDPOINT")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| { eprintln!("用法: azure_openai_chat <ENDPOINT> <API_KEY> [DEPLOYMENT]"); std::process::exit(1); });
    let api_key = std::env::var("AZURE_OPENAI_KEY")
        .or_else(|_| std::env::args().nth(2).ok_or(()))
        .unwrap_or_else(|_| { eprintln!("缺少 API_KEY"); std::process::exit(1); });
    let deployment = std::env::var("AZURE_DEPLOYMENT")
        .or_else(|_| std::env::args().nth(3).ok_or(()))
        .unwrap_or_else(|_| "gpt-4o".to_string());
    let prompt = std::env::args().nth(4).unwrap_or_else(|| "用一句话介绍一下你自己".to_string());

    let client = LlmClient::builder()
        .site(AzureOpenAiSite::new(&endpoint))
        .protocol(OpenAiProtocol {})
        .model_resolver(AzureOpenAiModelResolver::new())
        .with_extension(Arc::new(AzureOpenAiExtension::new()))
        .default_model(&deployment)
        .with_api_key(&api_key)
        .build();

    println!("========================================");
    println!("  Azure OpenAI 基础对话");
    println!("========================================\n");
    println!("Endpoint: {}", endpoint);
    println!("Deployment: {}", deployment);
    println!("用户: {}\n", prompt);

    let req = PrimitiveRequest::single_user_message(&prompt);
    match client.complete(&req).await {
        Ok(resp) => {
            println!("AI: {}", resp.content);
            if let Some(usage) = resp.usage {
                println!("\n[Token 用量: prompt={}, completion={}, total={}]",
                    usage.prompt_tokens, usage.completion_tokens, usage.total_tokens);
            }
        }
        Err(e) => { eprintln!("请求失败: {}", e); }
    }
    Ok(())
}
