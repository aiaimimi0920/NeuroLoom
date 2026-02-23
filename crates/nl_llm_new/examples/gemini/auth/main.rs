//! Gemini Provider - 认证与 API Key 验证示例
//!
//! 由于 Gemini 主要基于 API Key，而非 OAuth，此示例模拟验证密钥的有效性。
//!
//! 用法:
//!   cargo run --example gemini_auth -p nl_llm_new -- --key AIzaSy...

#[tokio::main]
async fn main() {
    println!("========================================");
    println!("  Gemini API Key Auth Check (nl_llm_new)");
    println!("========================================");

    let args: Vec<String> = std::env::args().collect();
    let api_key = args.windows(2).find(|w| w[0] == "--key").map(|w| w[1].clone())
        .or_else(|| std::env::var("GEMINI_API_KEY").ok());

    let api_key = match api_key {
        Some(key) => key,
        None => {
            eprintln!("===== 认证检查失败 =====");
            eprintln!("未找到 API Key。");
            eprintln!("请通过命令行参数 --key [YOUR_KEY] 或环境变量 GEMINI_API_KEY 提供。");
            return;
        }
    };

    println!("发现 API Key: {}...", &api_key[..12.min(api_key.len())]);
    println!("正在通过 Google AI Studio 验证 API Key 的有效性...");

    // 调用最轻量级的 models/list API 验证 Key 的有效性
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models?key={}",
        api_key
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap();
        
    match client.get(&url).send().await {
        Ok(res) => {
            let status = res.status();
            if status.is_success() {
                println!("===== 认证成功! =====");
                println!("API Key 有效，身份验证通过。");
            } else {
                println!("===== 认证失败 =====");
                println!("HTTP 状态码: {}", status);
                if let Ok(text) = res.text().await {
                    println!("错误详情: {}", text.trim());
                }
            }
        }
        Err(e) => {
            println!("===== 网络请求失败 =====");
            println!("Error: {:?}", e);
        }
    }
}
