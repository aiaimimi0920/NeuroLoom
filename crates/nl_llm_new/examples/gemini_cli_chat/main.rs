//! Gemini CLI 对话测试
//!
//! 使用已保存的 OAuth token 调用 cloudcode-pa.googleapis.com
//! (Gemini CLI 使用与 Antigravity 相同的 API 端点，但不同的 OAuth 凭据)
//!
//! 用法:
//!   cargo run --example gemini_cli_chat -p nl_llm_new
//!   cargo run --example gemini_cli_chat -p nl_llm_new -- "你好" --stream

use nl_llm_new::auth::providers::gemini_cli::GEMINI_CLI_OAUTH_CONFIG;
use serde::{Deserialize, Serialize};
use serde_json::json;

const BASE_URL: &str = "https://cloudcode-pa.googleapis.com";
const API_VERSION: &str = "v1internal";
const MODEL: &str = "gemini-2.5-flash";
// Gemini CLI 使用 nodejs 风格的 headers（区别于 Antigravity 的 python 风格）
const USER_AGENT: &str = "google-api-nodejs-client/9.15.1";
const API_CLIENT: &str = "gl-node/22.17.0";
// Gemini CLI 用逗号分隔格式（区别于 Antigravity 的 JSON 格式）
const CLIENT_METADATA: &str = "ideType=IDE_UNSPECIFIED,platform=PLATFORM_UNSPECIFIED,pluginType=GEMINI";

/// 从老 nl_llm 格式加载的 token
#[derive(Debug, Deserialize, Serialize)]
struct OldToken {
    access_token: String,
    refresh_token: String,
    #[serde(default)]
    project_id: Option<String>,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    expires_at: Option<String>,
}

fn token_path() -> std::path::PathBuf {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(home)
        .join(".nl_llm")
        .join("gemini_cli_token.json")
}

fn load_token() -> Result<OldToken, Box<dyn std::error::Error>> {
    let path = token_path();
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("无法读取 token 文件 {:?}: {}", path, e))?;
    let token: OldToken = serde_json::from_str(&content)?;
    Ok(token)
}

fn save_token(token: &OldToken) -> Result<(), Box<dyn std::error::Error>> {
    let path = token_path();
    let content = serde_json::to_string_pretty(token)?;
    std::fs::write(&path, content)?;
    Ok(())
}

async fn refresh_token(token: &mut OldToken) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let body = json!({
        "client_id": GEMINI_CLI_OAUTH_CONFIG.client_id,
        "client_secret": GEMINI_CLI_OAUTH_CONFIG.client_secret,
        "refresh_token": token.refresh_token,
        "grant_type": "refresh_token",
    });

    let resp = client
        .post(GEMINI_CLI_OAUTH_CONFIG.token_url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Token 刷新失败: {}", text).into());
    }

    #[derive(Deserialize)]
    struct RefreshResponse {
        access_token: String,
        expires_in: i64,
    }
    let refresh: RefreshResponse = resp.json().await?;

    token.access_token = refresh.access_token;
    let expires_at = chrono::Utc::now() + chrono::Duration::seconds(refresh.expires_in);
    token.expires_at = Some(expires_at.to_rfc3339());

    Ok(())
}

fn needs_refresh(token: &OldToken) -> bool {
    match &token.expires_at {
        Some(exp_str) => {
            if let Ok(exp) = chrono::DateTime::parse_from_rfc3339(exp_str) {
                exp <= chrono::Utc::now() + chrono::Duration::seconds(300)
            } else {
                true
            }
        }
        None => true,
    }
}

fn compile_request(prompt: &str, project_id: Option<&str>) -> serde_json::Value {
    let project = project_id
        .map(String::from)
        .unwrap_or_else(|| format!("project-{}", uuid::Uuid::new_v4()));

    json!({
        "model": MODEL,
        "userAgent": "gemini-cli",
        "requestType": "agent",
        "project": project,
        "requestId": format!("gemini-cli-{}", uuid::Uuid::new_v4()),
        "request": {
            "contents": [{
                "role": "user",
                "parts": [{ "text": prompt }]
            }]
        }
    })
}

async fn complete(
    token: &OldToken,
    prompt: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let body = compile_request(prompt, token.project_id.as_deref());
    let url = format!("{}/{}:generateContent", BASE_URL, API_VERSION);

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token.access_token))
        .header("Content-Type", "application/json")
        .header("User-Agent", USER_AGENT)
        .header("X-Goog-Api-Client", API_CLIENT)
        .header("Client-Metadata", CLIENT_METADATA)
        .json(&body)
        .send()
        .await?;

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(format!("API 请求失败 ({}): {}", status, text.trim()).into());
    }

    // Parse response - two possible formats:
    // { "response": { "candidates": [...] } }
    // { "candidates": [...] }
    let v: serde_json::Value = serde_json::from_str(&text)?;
    let candidates = v
        .get("response")
        .and_then(|r| r.get("candidates"))
        .or_else(|| v.get("candidates"));

    let reply = candidates
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("content"))
        .and_then(|c| c.get("parts"))
        .and_then(|p| p.get(0))
        .and_then(|p| p.get("text"))
        .and_then(|t| t.as_str())
        .unwrap_or("[无法解析响应]");

    Ok(reply.to_string())
}

async fn stream_complete(
    token: &OldToken,
    prompt: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let body = compile_request(prompt, token.project_id.as_deref());
    let url = format!("{}/{}:streamGenerateContent?alt=sse", BASE_URL, API_VERSION);

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token.access_token))
        .header("Content-Type", "application/json")
        .header("User-Agent", USER_AGENT)
        .header("X-Goog-Api-Client", API_CLIENT)
        .header("Client-Metadata", CLIENT_METADATA)
        .json(&body)
        .send()
        .await?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("流式请求失败 ({}): {}", status, text.trim()).into());
    }

    use futures::StreamExt;
    use std::io::Write;
    let mut stream = resp.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk) = stream.next().await {
        let bytes = chunk?;
        buffer.push_str(&String::from_utf8_lossy(&bytes));

        // Detect SSE event boundaries which could be \r\n\r\n or \n\n
        while let Some(pos) = buffer.find("\r\n\r\n").or_else(|| buffer.find("\n\n")) {
            let offset = if buffer[pos..].starts_with("\r\n\r\n") { 4 } else { 2 };
            let event = buffer[..pos].to_string();
            buffer = buffer[pos + offset..].to_string();

            for line in event.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        println!();
                        return Ok(());
                    }
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
                        let candidates = v
                            .get("response")
                            .and_then(|r| r.get("candidates"))
                            .or_else(|| v.get("candidates"));
                        if let Some(text) = candidates
                            .and_then(|c| c.get(0).or_else(|| c.as_array().and_then(|a| a.get(0))))
                            .and_then(|c| c.get("content"))
                            .and_then(|c| c.get("parts"))
                            .and_then(|p| p.get(0).or_else(|| p.as_array().and_then(|a| a.get(0))))
                            .and_then(|p| p.get("text"))
                            .and_then(|t| t.as_str())
                        {
                            print!("{}", text);
                            std::io::stdout().flush().unwrap();
                        }
                    }
                }
            }
        }
    }

    println!();
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let use_stream = args.iter().any(|a| a == "--stream");
    let prompt = args
        .iter()
        .skip(1)
        .find(|a| !a.starts_with("--"))
        .cloned()
        .unwrap_or_else(|| "你好！请用中文简单介绍一下你自己。".to_string());

    println!("========================================");
    println!("  Gemini CLI Chat Test (nl_llm_new)");
    println!("========================================");
    println!(
        "  模式: {}",
        if use_stream {
            "流式 (streamGenerateContent)"
        } else {
            "非流式 (generateContent)"
        }
    );
    println!("  模型: {}", MODEL);
    println!("========================================");
    println!();
    println!("用户: {}", prompt);
    println!();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        // 加载 token
        let mut token = match load_token() {
            Ok(t) => t,
            Err(e) => {
                eprintln!("无法加载 token: {}", e);
                eprintln!();
                eprintln!("提示: 请先通过 Gemini CLI 登录,");
                eprintln!("token 文件位置: {:?}", token_path());
                std::process::exit(1);
            }
        };

        // 刷新 token
        print!("正在验证身份... ");
        if needs_refresh(&token) {
            match refresh_token(&mut token).await {
                Ok(()) => {
                    let _ = save_token(&token);
                    println!(
                        "✓ 已刷新 (token: {}...)",
                        &token.access_token[..20.min(token.access_token.len())]
                    );
                }
                Err(e) => {
                    println!("✗");
                    eprintln!("Token 刷新失败: {}", e);
                    eprintln!("请删除 {:?} 后重新登录", token_path());
                    std::process::exit(1);
                }
            }
        } else {
            println!(
                "✓ (token: {}...)",
                &token.access_token[..20.min(token.access_token.len())]
            );
        }
        println!();

        println!("正在请求模型...");
        println!();

        if use_stream {
            match stream_complete(&token, &prompt).await {
                Ok(()) => {
                    println!();
                    println!("----------------------------------------");
                }
                Err(e) => {
                    eprintln!();
                    eprintln!("请求失败: {}", e);
                    std::process::exit(1);
                }
            }
        } else {
            match complete(&token, &prompt).await {
                Ok(reply) => {
                    println!("----------------------------------------");
                    println!("AI 回复:");
                    println!("----------------------------------------");
                    println!("{}", reply);
                    println!("----------------------------------------");
                }
                Err(e) => {
                    eprintln!();
                    eprintln!("请求失败: {}", e);
                    eprintln!();
                    eprintln!("排查建议:");
                    eprintln!("  1. 删除 {:?} 后重新登录", token_path());
                    eprintln!("  2. 检查是否能访问 cloudcode-pa.googleapis.com");
                    std::process::exit(1);
                }
            }
        }
    });
}
