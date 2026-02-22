//! iFlow 模型列表查询

use nl_llm_new::provider::iflow::config::IflowConfig;
use nl_llm_new::provider::iflow::provider::IflowProvider;
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let cookie = if args.len() > 1 {
        args[1].clone()
    } else {
        let config_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("examples")
            .join("iflow")
            .join("iflow_config.txt");

        match read_token_from_config(&config_path) {
            Some(t) => t,
            None => {
                eprintln!("请在 examples/iflow/iflow_config.txt 中配置 BXAuth");
                std::process::exit(1);
            }
        }
    };

    let provider = IflowProvider::new(IflowConfig::new(cookie, "qwen3-max".to_string()));

    println!("Getting iFlow API Key and model list...");
    println!();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        println!("Step 1: Getting API Key...");
        let api_key = match provider.fetch_api_key().await {
            Ok(key) => {
                println!("  API Key: {}...", &key[..8.min(key.len())]);
                key
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
