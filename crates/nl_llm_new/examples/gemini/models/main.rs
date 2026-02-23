//! Gemini Provider - 模型列表展示
//!
//! 通过 Google AI Studio API 动态获取当前账号通过 API Key 可以访问的模型列表。
//!
//! 用法:
//!   cargo run --example gemini_models -p nl_llm_new -- --key AIzaSy...

use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct GeminiModel {
    name: String,
    version: String,
    #[serde(rename = "displayName", default)]
    display_name: String,
    description: Option<String>,
    #[serde(rename = "supportedGenerationMethods", default)]
    supported_generation_methods: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct ModelsResponse {
    models: Vec<GeminiModel>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let api_key = args.windows(2).find(|w| w[0] == "--key").map(|w| w[1].clone())
        .or_else(|| std::env::var("GEMINI_API_KEY").ok())
        .unwrap_or_else(|| {
            eprintln!("请提供 API Key: --key AIzaSy... 或设置 GEMINI_API_KEY 环境变量");
            std::process::exit(1);
        });

    println!("========================================");
    println!("  Gemini Official Models (nl_llm_new)");
    println!("========================================");
    println!("  Key: {}...", &api_key[..12.min(api_key.len())]);
    println!("========================================");
    println!();

    println!("正在通过 Google AI Studio 动态请求可用模型列表...");
    
    // https://ai.google.dev/api/rest/v1beta/models/list
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models?key={}",
        api_key
    );

    let client = reqwest::Client::new();
    let res = client.get(&url).send().await?;

    let status = res.status();
    let text = res.text().await?;

    if !status.is_success() {
        println!("获取模型列表失败: {}", status);
        println!("响应: {}", text);
        return Ok(());
    }

    let response: ModelsResponse = serde_json::from_str(&text)?;

    println!();
    println!("Google AI Studio 支持的动态模型列表:");
    println!("----------------------------------------");
    
    // 过滤出支持 generateContent 或 streamGenerateContent 的大语言模型
    let chat_models: Vec<&GeminiModel> = response.models.iter()
        .filter(|m| m.supported_generation_methods.contains(&"generateContent".to_string()))
        .collect();

    for (index, model) in chat_models.iter().enumerate() {
        // 从 "models/gemini-pro" 等格式中提取短名称
        let short_name = model.name.strip_prefix("models/").unwrap_or(&model.name);
        println!("{:2}. {:<25} - {}", index + 1, short_name, model.display_name);
    }
    
    println!("----------------------------------------");
    println!("(共找到 {} 个支持 generateContent 的模型)", chat_models.len());

    Ok(())
}
