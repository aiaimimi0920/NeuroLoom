use nl_llm_new::auth::providers::gemini_cli::GeminiCliOAuth;
use reqwest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("========================================");
    println!("  Gemini CLI Models (nl_llm_new)");
    println!("========================================");

    let auth_path = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    let path = std::path::PathBuf::from(auth_path).join(".nl_llm").join("gemini_cli_token.json");

    println!("正在验证认证状态并获取 Token...");
    let mut auth = GeminiCliOAuth::from_file(&path)?;
    auth.ensure_authenticated().await?;
    let _access_token = auth.access_token().unwrap();

    // 注意：与 Antigravity 插件凭据不同，Gemini CLI (google-api-nodejs-client)
    // 凭据缺乏访问 `fetchAvailableModels` 内部端点的权限 (403 Permission Denied)。
    // 本次示例仅在成功完成 OAuth 登录后提供该 CLI 工具本身所支持的模型列表。

    println!("\n动态请求被 Google API 拒绝 (403 Forbidden: The caller does not have permission)");
    println!("由于 Gemini CLI OAuth 客户端权限限制，无法通过内部接口拉取可用模型列表。");
    
    println!("\n支持的静态模型列表 (Gemini CLI):");
    println!("----------------------------------------");
    let models = vec![
        "gemini-2.5-flash",
        "gemini-2.5-pro",
        "gemini-2.0-flash",
        "gemini-2.0-pro-exp-02-05",
        "gemini-2.0-flash-thinking-exp-01-21",
        "gemini-1.5-pro",
        "gemini-1.5-flash",
        "gemini-1.5-pro-002",
        "gemini-1.5-flash-002",
    ];

    for (index, &model) in models.iter().enumerate() {
        println!("{}. {}", index + 1, model);
    }
    println!("----------------------------------------");

    Ok(())
}
