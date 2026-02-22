//! iFlow 认证测试 (Cookie → API Key)

use nl_llm_new::provider::iflow::config::IFlowConfig;
use nl_llm_new::provider::iflow::provider::IFlowProvider;
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
                eprintln!("用法: {} <BXAuth_COOKIE>", args[0]);
                eprintln!("或在 examples/iflow/iflow_config.txt 中配置 BXAuth");
                std::process::exit(1);
            }
        }
    };

    println!("========================================");
    println!("  iFlow Auth Test (nl_llm_new)");
    println!("========================================");
    println!();
    println!("Cookie: {}...", &cookie[..20_usize.min(cookie.len())]);
    println!();

    let provider = IFlowProvider::new(IFlowConfig::new(cookie, "qwen3-max".to_string()));

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        println!("正在获取 API Key (2-step: GET info → POST refresh)...");
        match provider.fetch_api_key().await {
            Ok(api_key) => {
                println!();
                println!("===== 认证成功! =====");
                println!("API Key: {}...{}", &api_key[..8_usize.min(api_key.len())], &api_key[api_key.len().saturating_sub(4)..]);
            }
            Err(e) => {
                println!("===== 认证失败 =====");
                println!("Error: {:?}", e);
                println!();
                println!("请检查:");
                println!("  1. BXAuth Cookie 是否有效");
                println!("  2. platform.iflow.cn 是否可访问");
            }
        }
    });

    println!();
    println!("按 Enter 退出...");
    let _ = std::io::stdin().read_line(&mut String::new());
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
