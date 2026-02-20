//! iFlow 认证测试示例
//!
//! 从 iflow_config.txt 读取配置，或通过命令行参数传入

use nl_llm::provider::{IFlowConfig, IFlowProvider};
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // 尝试从配置文件或命令行获取 Token
    let token = if args.len() > 1 {
        args[1].clone()
    } else {
        // 从配置文件读取
        let config_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("examples")
            .join("iflow_config.txt");

        match read_token_from_config(&config_path) {
            Some(t) => t,
            None => {
                eprintln!("用法: {} <BXAuth_COOKIE>", args[0]);
                eprintln!("或在 examples/iflow_config.txt 中配置 BXAuth");
                std::process::exit(1);
            }
        }
    };

    let config = IFlowConfig {
        cookie: token,
        model: "qwen3-max".to_string(),
        api_key: None,
        expire_time: None,
        email: None,
    };

    let mut provider = IFlowProvider::new(config);

    println!("Testing iFlow authentication...");
    println!();

    // 使用 tokio 运行异步代码
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        match provider.refresh_api_key().await {
            Ok(result) => {
                println!("Authentication successful!");
                println!();
                println!("API Key: {}", result.api_key);
                println!("Email: {}", result.email);
                println!("Expires: {}", result.expire_time);
                println!("Needs refresh within 2 days: {}", result.needs_refresh);
                println!();

                // 尝试保存 token
                if let Some(storage) = provider.to_storage() {
                    let temp_path = std::env::temp_dir().join("iflow_token.json");
                    match storage.save_to_file(&temp_path) {
                        Ok(_) => println!("Token saved to: {:?}", temp_path),
                        Err(e) => println!("Failed to save token: {:?}", e),
                    }
                }
            }
            Err(e) => {
                println!("Authentication failed!");
                println!("Error: {:?}", e);
            }
        }
    });
}

/// 从配置文件读取 BXAuth Token
fn read_token_from_config(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("BXAuth=") {
            return Some(line.to_string());
        }
    }

    None
}
