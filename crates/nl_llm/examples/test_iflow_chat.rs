//! iFlow 聊天测试示例
//!
//! 用法:
//!   test_iflow_chat.exe [prompt] [model]
//!
//! 配置文件: examples/iflow_config.txt

use nl_llm::provider::{IFlowConfig, IFlowProvider};
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // 从配置文件读取配置
    let config_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("iflow_config.txt");

    let (token, default_prompt, default_model) = read_config(&config_path);

    // 命令行参数覆盖配置文件
    let prompt = if args.len() > 1 { &args[1] } else { &default_prompt };
    let model = if args.len() > 2 { &args[2] } else { &default_model };

    if token.is_empty() {
        eprintln!("请在 examples/iflow_config.txt 中配置 BXAuth");
        std::process::exit(1);
    }

    let config = IFlowConfig {
        cookie: token,
        model: model.to_string(),
        api_key: None,
        expire_time: None,
        email: None,
    };

    let mut provider = IFlowProvider::new(config);

    println!("========================================");
    println!("iFlow Chat Test");
    println!("========================================");
    println!("Model: {}", model);
    println!("Prompt: {}", prompt);
    println!();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        run_chat(&mut provider, prompt, model).await;
    });
}

async fn run_chat(provider: &mut IFlowProvider, prompt: &str, model: &str) {
    // 刷新 API Key
    print!("Authenticating... ");
    match provider.refresh_api_key().await {
        Ok(result) => {
            println!("OK");
            println!("  API Key: {}...", &result.api_key[..20.min(result.api_key.len())]);
            println!("  Expires: {}", result.expire_time);
        }
        Err(e) => {
            println!("FAILED");
            eprintln!("Error: {:?}", e);
            return;
        }
    }
    println!();

    // 发送请求
    print!("Requesting model... ");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .unwrap();

    let api_key = match provider.get_api_key() {
        Some(k) => k.to_string(),
        None => {
            eprintln!("No API key available");
            return;
        }
    };

    let body = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "user", "content": prompt}
        ],
        "stream": false
    });

    let resp = client
        .post("https://apis.iflow.cn/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await;

    match resp {
        Ok(resp) => {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_else(|_| "Failed to read response".to_string());

            if status.is_success() {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Some(content) = json["choices"][0]["message"]["content"].as_str() {
                        println!("OK");
                        println!();
                        println!("----------------------------------------");
                        println!("AI Response:");
                        println!("----------------------------------------");
                        println!("{}", content);
                        println!("----------------------------------------");

                        if let Some(usage) = json.get("usage") {
                            println!();
                            println!("Token Usage:");
                            println!("  Input: {} tokens", usage["prompt_tokens"].as_u64().unwrap_or(0));
                            println!("  Output: {} tokens", usage["completion_tokens"].as_u64().unwrap_or(0));
                            println!("  Total: {} tokens", usage["total_tokens"].as_u64().unwrap_or(0));
                        }
                    } else {
                        println!("Failed to parse response");
                        println!("Raw: {}", text);
                    }
                }
            } else {
                println!("FAILED (status: {})", status);
                println!("Response: {}", text);
            }
        }
        Err(e) => {
            println!("FAILED");
            eprintln!("Error: {:?}", e);
        }
    }
}

/// 从配置文件读取配置
fn read_config(path: &Path) -> (String, String, String) {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return (String::new(), "Hello".to_string(), "qwen3-max".to_string()),
    };

    let mut token = String::new();
    let mut prompt = "Hello".to_string();
    let mut model = "qwen3-max".to_string();

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("BXAuth=") {
            token = line.to_string();
        } else if line.starts_with("MODEL=") {
            model = line[6..].to_string();
        } else if line.starts_with("PROMPT=") {
            prompt = line[7..].to_string();
        }
    }

    (token, prompt, model)
}
