//! iFlow 平台测试 - models
//!
//! 查询当前账户在 iFlow 平台上可用的模型列表
//!
//! 运行方式: cargo run -p nl_llm_v2 --example iflow_models

use anyhow::Result;
use std::path::Path;

use nl_llm_v2::LlmClient;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // 1. 获取 Cookie
    let cookie = args.get(1).cloned()
        .or_else(|| std::env::var("IFLOW_COOKIE").ok())
        .or_else(|| read_cookie_from_config())
        .expect("请提供 iFlow Cookie: 命令行参数 / IFLOW_COOKIE 环境变量 / examples/iflow/iflow_config.txt");

    println!("=== iFlow 模型列表查询 ===\n");

    // 2. 初始化客户端以获取 Authenticator 及其挂载的 ExtensionApi
    let client = LlmClient::from_preset("iflow")
        .expect("iflow preset should exist")
        .with_cookie(&cookie)
        .build();

    println!("正在获取可用模型列表...");
    let models = client.list_models().await?;

    println!("\n可用模型列表:");
    println!("----------------------------------------");
    for model in &models {
        println!("  - {}", model.id);
    }
    println!("----------------------------------------");
    println!("共计 {} 个模型", models.len());

    Ok(())
}

fn read_cookie_from_config() -> Option<String> {
    let paths = [
        "crates/nl_llm_v2/examples/iflow/iflow_config.txt",
        "examples/iflow/iflow_config.txt",
    ];
    for p in &paths {
        if let Some(cookie) = try_read_config(Path::new(p)) {
            return Some(cookie);
        }
    }
    None
}

fn try_read_config(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("BXAuth=") {
            return Some(line.to_string());
        }
    }
    None
}
