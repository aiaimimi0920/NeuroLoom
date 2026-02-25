//! Azure OpenAI 工具调用示例
//!
//! 运行方式:
//!   set AZURE_OPENAI_API_KEY=your-key
//!   set AZURE_OPENAI_ENDPOINT=https://YOUR-RESOURCE.openai.azure.com
//!   cargo run -p nl_llm_v2 --example azure_openai_tools

use nl_llm_v2::{LlmClient, primitive::tool::PrimitiveTool, PrimitiveRequest};
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("AZURE_OPENAI_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: azure_openai_tools <API_KEY> <ENDPOINT> <DEPLOYMENT>");
            eprintln!("或设置环境变量:");
            eprintln!("  AZURE_OPENAI_API_KEY");
            eprintln!("  AZURE_OPENAI_ENDPOINT");
            std::process::exit(1);
        });

    let endpoint = std::env::var("AZURE_OPENAI_ENDPOINT")
        .or_else(|_| std::env::args().nth(2).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("错误: 需要设置 AZURE_OPENAI_ENDPOINT 环境变量");
            std::process::exit(1);
        });

    let deployment = std::env::args().nth(3).unwrap_or_else(|| "gpt-4o".to_string());
    let prompt = std::env::args().nth(4).unwrap_or_else(|| {
        "北京和上海今天的天气怎么样？".to_string()
    });

    let client = LlmClient::from_preset("azure_openai")
        .expect("Preset should exist")
        .with_base_url(&endpoint)
        .with_api_key(&api_key)
        .build();

    println!("========================================");
    println!("  Azure OpenAI 工具调用");
    println!("========================================\n");
    println!("Endpoint: {}", endpoint);
    println!("Deployment: {}", deployment);
    println!("用户: {}\n", prompt);

    let mut req = PrimitiveRequest::single_user_message(&prompt)
        .with_model(&deployment);

    req.tools = vec![
        PrimitiveTool {
            name: "get_weather".to_string(),
            description: Some("获取指定城市的当前天气信息".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "city": {
                        "type": "string",
                        "description": "城市名称"
                    }
                },
                "required": ["city"]
            }),
        },
    ];

    println!("可用工具:");
    for tool in &req.tools {
        println!("  • {} — {}", tool.name, tool.description.as_deref().unwrap_or(""));
    }
    println!();

    match client.complete(&req).await {
        Ok(resp) => {
            println!("AI 响应:");
            println!("{}", resp.content);

            if let Some(usage) = &resp.usage {
                println!("\n[Token 用量: prompt={}, completion={}, total={}]",
                    usage.prompt_tokens, usage.completion_tokens, usage.total_tokens);
            }
        }
        Err(e) => {
            eprintln!("请求失败: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
