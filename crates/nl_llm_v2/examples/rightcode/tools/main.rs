//! RightCode 工具调用示例
//!
//! 演示如何使用 RightCode 进行工具调用 (Function Calling)
//!
//! 运行方式:
//!   - 设置环境变量: set RIGHTCODE_API_KEY=your-key
//!   - 运行: cargo run -p nl_llm_v2 --example rightcode_tools

use nl_llm_v2::{LlmClient, primitive::tool::PrimitiveTool, PrimitiveRequest};
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("RIGHTCODE_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("用法: rightcode_tools <API_KEY>");
            eprintln!("或设置 RIGHTCODE_API_KEY 环境变量");
            std::process::exit(1);
        });

    let prompt = std::env::args().nth(2).unwrap_or_else(|| {
        "北京和上海今天的天气怎么样？".to_string()
    });

    let client = LlmClient::from_preset("rightcode")
        .expect("Preset should exist")
        .with_api_key(&api_key)
        .build();

    println!("========================================");
    println!("  RightCode 工具调用");
    println!("========================================\n");
    println!("模型: gpt-5.1-codex-mini (支持工具调用)");
    println!("用户: {}\n", prompt);

    // 构建带工具的请求
    let mut req = PrimitiveRequest::single_user_message(&prompt)
        .with_model("gpt-5.1-codex-mini");

    req.tools = vec![
        PrimitiveTool {
            name: "get_weather".to_string(),
            description: Some("获取指定城市的当前天气信息".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "city": {
                        "type": "string",
                        "description": "城市名称，如：北京、上海"
                    },
                    "unit": {
                        "type": "string",
                        "enum": ["celsius", "fahrenheit"],
                        "description": "温度单位"
                    }
                },
                "required": ["city"]
            }),
        },
        PrimitiveTool {
            name: "get_time".to_string(),
            description: Some("获取指定城市的当前时间".to_string()),
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

    println!("发送请求...\n");

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

    println!("\n========================================");
    println!("  说明");
    println!("========================================");
    println!("工具调用结果会被模型处理并生成自然语言响应。");
    println!("如果模型决定调用工具，会在内部处理后返回结果。");

    Ok(())
}
