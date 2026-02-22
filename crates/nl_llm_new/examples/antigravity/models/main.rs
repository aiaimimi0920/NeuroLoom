//! Antigravity (Gemini Code Assist) 模型列表展示
//!
//! 由于 Cloud Code (PA) 端点没有开放的 `/models` 查询接口，
//! 此代码展示后端原生支持的 Antigravity Gemini 模型清单。

use nl_llm_new::auth::providers::antigravity::AntigravityOAuth;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("========================================");
    println!("  Antigravity Models (nl_llm_new)");
    println!("========================================");
    println!();

    let mut path = PathBuf::from(std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\Public".to_string()));
    path.push(".nl_llm");
    path.push("antigravity_token.json");

    let mut auth = AntigravityOAuth::from_file(&path)?;
    println!("正在验证认证状态并获取 Token...");
    auth.ensure_authenticated().await?;
    let access_token = auth.access_token().unwrap();

    let url = "https://cloudcode-pa.googleapis.com/v1internal:fetchAvailableModels";
    let body = serde_json::json!({});

    println!("正在动态请求可用模型列表...");
    let client = reqwest::Client::new();
    let res = client
        .post(url)
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Content-Type", "application/json")
        .header("User-Agent", "antigravity/1.104.0 darwin/arm64")
        .json(&body)
        .send()
        .await?;

    let status = res.status();
    let text = res.text().await?;

    if !status.is_success() {
        println!("获取模型列表失败: {}", status);
        println!("响应: {}", text);
        return Ok(());
    }

    let json: serde_json::Value = serde_json::from_str(&text)?;
    let models = json.get("models").and_then(|v| v.as_object());

    println!();
    println!("Google Cloud Code (PA) 支持的动态模型列表:");
    println!("----------------------------------------");
    if let Some(models) = models {
        for (model_name, _) in models {
            println!("  - {}", model_name);
        }
    } else {
        println!("未能解析出模型列表。");
    }
    println!("----------------------------------------");

    Ok(())
}
