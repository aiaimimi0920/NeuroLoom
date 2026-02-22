//! Antigravity (Gemini Code Assist) 对话测试
//!
//! 使用已保存的 OAuth token 调用 cloudcode-pa.googleapis.com
//!
//! 用法:
//!   cargo run --example antigravity_chat -p nl_llm_new
//!   cargo run --example antigravity_chat -p nl_llm_new -- "你好" --stream

const BASE_URL: &str = "https://cloudcode-pa.googleapis.com";
const API_VERSION: &str = "v1internal";
const MODEL: &str = "gemini-2.5-flash";
const USER_AGENT: &str = "google-cloud-sdk gcloud/0.0.0.dev";
const API_CLIENT: &str = "gl-python/3.12.0";

use nl_llm_new::auth::providers::antigravity::AntigravityOAuth;

fn generate_project_id() -> String {
    let adjectives = ["useful", "bright", "swift", "calm", "bold"];
    let nouns = ["fuze", "wave", "spark", "flow", "core"];
    let uid = uuid::Uuid::new_v4().to_string();
    let random_part = &uid[..5];
    let nanos = chrono::Utc::now().timestamp_subsec_nanos() as usize;
    let adj = adjectives[nanos % adjectives.len()];
    let noun = nouns[(nanos / 2) % nouns.len()];
    format!("{}-{}-{}", adj, noun, random_part)
}

fn compile_request(prompt: &str, project_id: Option<&str>) -> serde_json::Value {
    let project = match project_id {
        Some(pid) if !pid.is_empty() => pid.to_string(),
        _ => generate_project_id(),
    };

    serde_json::json!({
        "model": MODEL,
        "userAgent": "antigravity",
        "requestType": "agent",
        "project": project,
        "requestId": format!("agent-{}", uuid::Uuid::new_v4()),
        "request": {
            "contents": [{
                "role": "user",
                "parts": [{ "text": prompt }]
            }]
        }
    })
}

async fn complete(
    access_token: &str,
    project_id: Option<&str>,
    prompt: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let body = compile_request(prompt, project_id);
    let url = format!("{}/{}:generateContent", BASE_URL, API_VERSION);

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Content-Type", "application/json")
        .header("User-Agent", USER_AGENT)
        .header("X-Goog-Api-Client", API_CLIENT)
        .header(
            "Client-Metadata",
            r#"{"ideType":"IDE_UNSPECIFIED","platform":"PLATFORM_UNSPECIFIED","pluginType":"GEMINI"}"#,
        )
        .json(&body)
        .send()
        .await?;

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(format!("API 请求失败 ({}): {}", status, text.trim()).into());
    }

    // Parse response - Antigravity has two formats:
    // Format 1: { "response": { "candidates": [...] } }
    // Format 2: { "candidates": [...] }
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
        .unwrap_or_else(|| {
            eprintln!("DEBUG: 无法解析响应: {}", &text[..500.min(text.len())]);
            "[无法解析响应]"
        });

    Ok(reply.to_string())
}

async fn stream_complete(
    access_token: &str,
    project_id: Option<&str>,
    prompt: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let body = compile_request(prompt, project_id);
    let url = format!("{}/{}:streamGenerateContent?alt=sse", BASE_URL, API_VERSION);

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Content-Type", "application/json")
        .header("User-Agent", USER_AGENT)
        .header("X-Goog-Api-Client", API_CLIENT)
        .header(
            "Client-Metadata",
            r#"{"ideType":"IDE_UNSPECIFIED","platform":"PLATFORM_UNSPECIFIED","pluginType":"GEMINI"}"#,
        )
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
        .unwrap_or_else(|| "你好！请用中文简单介绍一下你自己，以及你能做什么？".to_string());

    println!("========================================");
    println!("  Antigravity (Gemini Code Assist) Chat");
    println!("========================================");
    println!(
        "  模式: {}",
        if use_stream {
            "流式 (streamGenerateContent)"
        } else {
            "非流式 (generateContent)"
        }
    );
    println!("========================================");
    println!();
    println!("用户: {}", prompt);
    println!();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".to_string());
        let token_path = std::path::PathBuf::from(home)
            .join(".nl_llm")
            .join("antigravity_token.json");

        let mut auth = match AntigravityOAuth::from_file(&token_path) {
            Ok(a) => a,
            Err(e) => {
                eprintln!("初始化 Auth 失败: {:?}", e);
                std::process::exit(1);
            }
        };

        print!("正在验证身份... ");
        if let Err(e) = auth.ensure_authenticated().await {
            println!("✗");
            eprintln!("Token 刷新/登录失败: {:?}", e);
            eprintln!("请删除 {:?} 后重试", token_path);
            std::process::exit(1);
        }
        let access_token = auth.access_token().unwrap();
        let project_id = auth.token.as_ref().and_then(|t| t.project_id.as_deref());
        println!(
            "✓ (token: {}...)",
            &access_token[..20.min(access_token.len())]
        );
        println!();

        println!("正在请求模型...");
        println!();

        if use_stream {
            match stream_complete(access_token, project_id, &prompt).await {
                Ok(()) => {
                    println!();
                    println!("----------------------------------------");
                }
                Err(e) => {
                    eprintln!();
                    eprintln!("请求失败: {:?}", e);
                    std::process::exit(1);
                }
            }
        } else {
            match complete(access_token, project_id, &prompt).await {
                Ok(reply) => {
                    println!("----------------------------------------");
                    println!("AI 回复:");
                    println!("----------------------------------------");
                    println!("{}", reply);
                    println!("----------------------------------------");
                }
                Err(e) => {
                    eprintln!();
                    eprintln!("请求失败: {:?}", e);
                    eprintln!();
                    eprintln!("排查建议:");
                    eprintln!("  1. 检查账户是否有 Gemini Code Assist 订阅");
                    eprintln!("  2. 删除 {:?} 后重新登录", token_path);
                    eprintln!("  3. 检查是否能访问 cloudcode-pa.googleapis.com");
                    std::process::exit(1);
                }
            }
        }
    });
}
