//! ModelScope (魔搭) 认证验证测试

use nl_llm_v2::{LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("MODELSCOPE_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: modelscope_auth <ACCESS_TOKEN>");
            eprintln!("或设置 MODELSCOPE_API_KEY 环境变量");
            std::process::exit(1);
        });

    println!("========================================");
    println!("  ModelScope (魔搭) 认证验证");
    println!("========================================\n");
    println!("网关: https://api-inference.modelscope.cn/v1");

    if api_key.len() > 8 {
        println!("Token: {}...{}", &api_key[..4], &api_key[api_key.len().saturating_sub(4)..]);
    }

    let client = LlmClient::from_preset("modelscope")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    println!("\n可用模型 (静态列表):");
    match client.list_models().await {
        Ok(models) => { for m in &models { println!("  • {} — {}", m.id, m.description); } }
        Err(e) => println!("  获取失败: {}", e),
    }

    // 并发配置
    if let Some(ext) = client.extension() {
        let config = ext.concurrency_config();
        println!("\n并发配置:");
        println!("  官方上限: {}", config.official_max);
        println!("  初始并发: {}", config.initial_limit);
    }

    println!("\n尝试基础通信...");
    let req = PrimitiveRequest::single_user_message("Say 'auth ok' in exactly 2 words");
    match client.complete(&req).await {
        Ok(resp) => {
            println!("\n✅ 认证通讯成功！");
            println!("模型响应: {}", resp.content);
            if let Some(usage) = &resp.usage {
                println!("\nToken 用量: prompt={}, completion={}, total={}",
                    usage.prompt_tokens, usage.completion_tokens, usage.total_tokens);
            }
        }
        Err(e) => {
            println!("\n❌ 认证通讯失败: {}", e);
            println!("\n获取 Token: https://modelscope.cn/my/myaccesstoken");
            std::process::exit(1);
        }
    }
    Ok(())
}
