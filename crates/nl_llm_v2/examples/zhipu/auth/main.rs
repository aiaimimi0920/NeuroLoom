//! 智谱 BigModel (GLM国内版) 认证验证测试

use nl_llm_v2::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("ZHIPU_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: zhipu_auth <API_KEY>");
            eprintln!("或设置 ZHIPU_API_KEY 环境变量");
            std::process::exit(1);
        });

    println!("========================================");
    println!("  智谱 BigModel (GLM) 认证验证");
    println!("========================================\n");

    println!("API Key: {}...{}", &api_key[..4.min(api_key.len())], &api_key[api_key.len().saturating_sub(4)..]);

    let client = LlmClient::from_preset("zhipu")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    let req = PrimitiveRequest::single_user_message("Say 'auth ok'")
        .with_model("glm-5");

    match client.complete(&req).await {
        Ok(resp) => {
            println!("\n✅ 认证成功！");
            println!("模型响应: {}", resp.content);
            if let Some(usage) = &resp.usage {
                println!("Token 使用: prompt={}, completion={}, total={}",
                    usage.prompt_tokens, usage.completion_tokens, usage.total_tokens);
            }
        }
        Err(e) => {
            println!("\n❌ 认证失败: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
