//! ocoolAI 平台基础对话测试
//!
//! 运行方式:
//! - 直接运行 test.bat
//! - 或: cargo run --example ocoolai_chat -- <api_key> [prompt]
//!
//! 环境变量:
//! - OCOOLAI_API_KEY: API Key（可选，可从 .env.local 读取）
//! - OCOOLAI_BASE_URL: 自定义端点（可选）

use anyhow::Result;
use nl_llm_v2::{LlmClient, PrimitiveRequest};

fn load_env_from_file() {
    // 尝试从 .env.local 加载
    let env_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("ocoolai")
        .join(".env.local");

    if env_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&env_path) {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((key, value)) = line.split_once('=') {
                    // 只有环境变量不存在时才设置
                    if std::env::var(key.trim()).is_err() {
                        std::env::set_var(key.trim(), value.trim());
                    }
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // 加载本地配置
    load_env_from_file();

    let args: Vec<String> = std::env::args().collect();

    // 获取 API Key
    let api_key = std::env::var("OCOOLAI_API_KEY")
        .or_else(|_| args.get(1).cloned().ok_or_else(|| anyhow::anyhow!("Missing API key parameter")))
        .expect("需要提供 API Key (设置 OCOOLAI_API_KEY 环境变量或作为第一个参数传入)");

    // 获取 prompt
    let prompt = args.get(2).cloned().unwrap_or_else(|| {
        "你好！请简单介绍一下你自己，并告诉我你能提供哪些帮助。".to_string()
    });

    println!("========================================");
    println!("  ocoolAI Chat Test");
    println!("========================================");
    println!();

    // 创建客户端
    let mut builder = LlmClient::from_preset("ocoolai")
        .expect("ocoolAI preset should exist")
        .with_api_key(&api_key);

    // 检查是否有自定义 base_url
    if let Ok(base_url) = std::env::var("OCOOLAI_BASE_URL") {
        builder = builder.with_base_url(&base_url);
    }

    let client = builder.build();

    // 构建请求
    let req = PrimitiveRequest::single_user_message(&prompt).with_model("4o-mini");

    println!("用户: {}\n", prompt);
    println!("AI: ");

    // 发送请求
    let resp = client.complete(&req).await?;

    println!("{}", resp.content);

    if let Some(usage) = resp.usage {
        println!();
        println!(
            "[Token 用量: prompt={}, completion={}, total={}]",
            usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
        );
    }

    println!();
    println!("========================================");
    println!("  Test Complete");
    println!("========================================");

    Ok(())
}
