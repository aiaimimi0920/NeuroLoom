//! iFlow 模型列表查询
//!
//! 从 iflow_config.txt 读取配置

use nl_llm::provider::{IFlowConfig, IFlowProvider};
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // 从配置文件或命令行获取 Token
    let token = if args.len() > 1 {
        args[1].clone()
    } else {
        let config_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("examples")
            .join("iflow_config.txt");

        match read_token_from_config(&config_path) {
            Some(t) => t,
            None => {
                eprintln!("请在 examples/iflow_config.txt 中配置 BXAuth");
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

    println!("Getting iFlow API Key and model list...");
    println!();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        // 刷新 API Key
        println!("Step 1: Refreshing API Key...");
        let api_key = match provider.refresh_api_key().await {
            Ok(result) => {
                println!("  API Key: {}", result.api_key);
                println!("  Email: {}", result.email);
                println!("  Expires: {}", result.expire_time);
                result.api_key
            }
            Err(e) => {
                println!("  Failed: {:?}", e);
                return;
            }
        };
        println!();

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap();

        // 尝试获取模型列表
        println!("Step 2: Getting model list...");

        let resp = client
            .get("https://apis.iflow.cn/v1/models")
            .header("Authorization", format!("Bearer {}", &api_key))
            .send()
            .await;

        match resp {
            Ok(resp) => {
                println!("  Status: {}", resp.status());
                let text = resp.text().await.unwrap_or_else(|_| "Failed to read body".to_string());

                // 尝试解析模型列表
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                        println!();
                        println!("Available Models:");
                        println!("----------------------------------------");
                        for model in data {
                            if let Some(id) = model.get("id").and_then(|i| i.as_str()) {
                                println!("  - {}", id);
                            }
                        }
                        println!("----------------------------------------");
                    } else {
                        println!("  Response: {}", text);
                    }
                } else {
                    println!("  Response: {}", text);
                }
            }
            Err(e) => {
                println!("  Error: {:?}", e);
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
