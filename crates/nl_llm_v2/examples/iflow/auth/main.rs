//! iFlow 平台测试 - 认证
//!
//! 验证 Cookie 认证流程，展示 token 获取和签名机制
//!
//! 运行方式: cargo run -p nl_llm_v2 --example iflow_auth
//! 或直接运行: examples/iflow/auth/test.bat

use nl_llm_v2::{LlmClient, PrimitiveRequest};
use anyhow::Result;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let cookie = args.get(1).cloned()
        .or_else(|| std::env::var("IFLOW_COOKIE").ok())
        .or_else(|| read_cookie_from_config())
        .expect("请提供 iFlow Cookie: 命令行参数 / IFLOW_COOKIE 环境变量 / examples/iflow/iflow_config.txt");

    println!("=== iFlow 认证测试 ===\n");
    println!("Cookie: {}...", &cookie[..cookie.len().min(30)]);

    let client = LlmClient::from_preset("iflow")
        .expect("iflow preset should exist")
        .with_cookie(&cookie)
        .build();

    // 发送一个最简请求来验证认证
    let req = PrimitiveRequest::single_user_message("说'认证成功'两个字")
        .with_model("glm-4-flash");

    println!("\n发送测试请求...");
    match client.complete(&req).await {
        Ok(resp) => {
            println!("✅ 认证成功！");
            println!("  模型响应: {}", resp.content);
        }
        Err(e) => {
            println!("❌ 认证失败: {}", e);
        }
    }

    Ok(())
}

/// 从 iflow_config.txt 配置文件读取 BXAuth Cookie
fn read_cookie_from_config() -> Option<String> {
    // 尝试多个可能的路径
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
