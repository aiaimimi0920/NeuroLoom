//! Azure OpenAI 认证验证
//!
//! 需要环境变量或命令行参数：
//! - 参数1: Azure endpoint (如 https://myresource.openai.azure.com)
//! - 参数2: API key
//! - 参数3: Deployment name (可选，默认 gpt-4o)

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
        .unwrap_or_else(|_| {
            eprintln!("用法: azure_openai_auth <ENDPOINT> <API_KEY> [DEPLOYMENT]");
            eprintln!("  ENDPOINT: https://YOUR-RESOURCE.openai.azure.com");
            eprintln!("  API_KEY:  你的 Azure API Key");
            eprintln!("  DEPLOYMENT: 你的模型部署名 (默认: gpt-4o)");
            std::process::exit(1);
        });

    let api_key = std::env::var("AZURE_OPENAI_KEY")
        .or_else(|_| std::env::args().nth(2).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("缺少 API_KEY 参数");
            std::process::exit(1);
        });

    let deployment = std::env::var("AZURE_DEPLOYMENT")
        .or_else(|_| std::env::args().nth(3).ok_or(()))
        .unwrap_or_else(|_| "gpt-4o".to_string());

    println!("========================================");
    println!("  Azure OpenAI 认证验证");
    println!("========================================\n");
    println!("Endpoint: {}", endpoint);
    println!("Deployment: {}", deployment);

    // Azure 需要 api-key header 而不是 Authorization: Bearer
    let client = LlmClient::builder()
        .site(AzureOpenAiSite::new(&endpoint))
        .protocol(OpenAiProtocol {})
        .model_resolver(AzureOpenAiModelResolver::new())
        .with_extension(Arc::new(AzureOpenAiExtension::new()))
        .default_model(&deployment)
        .with_api_key(&api_key)
        .build();

    println!("\n尝试基础通信 (deployment: {})...", deployment);
    let req = PrimitiveRequest::single_user_message("Say 'auth ok' in exactly 2 words");
    match client.complete(&req).await {
        Ok(resp) => {
            println!("\n✅ 认证通讯成功！");
            println!("模型响应: {}", resp.content);
            if let Some(usage) = resp.usage {
                println!("[Token 用量: prompt={}, completion={}, total={}]",
                    usage.prompt_tokens, usage.completion_tokens, usage.total_tokens);
            }
        }
        Err(e) => { println!("\n❌ 认证通讯失败: {}", e); }
    }
    Ok(())
}
