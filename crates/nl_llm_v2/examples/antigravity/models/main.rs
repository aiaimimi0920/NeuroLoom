//! antigravity 平台测试 - models
//!
//! 查询 CloudCode PA 项目配置，并探测 Gemini 模型在 CloudCode 上的可用性
//!
//! 模型候选列表参照 CLIProxyAPI_Reference 中的 GetGeminiCLIModels() 静态定义
//!
//! 运行方式: cargo run --example antigravity_models
//! 或直接运行: test.bat
//!
//! 注意：此示例需要先运行一次 auth 或 chat 完成 OAuth 登录，
//!       生成 token 缓存文件后方可使用

use anyhow::Result;
use uuid::Uuid;

/// 参照 CLIProxyAPI_Reference/internal/registry/model_definitions_static_data.go
/// GetGeminiCLIModels() 中定义的标准 Gemini CLI / Antigravity 可用模型列表
const CANDIDATE_MODELS: &[(&str, &str)] = &[
    // ── 正式版 ──
    ("gemini-2.5-pro",            "Gemini 2.5 Pro — 最强推理 (1M token)"),
    ("gemini-2.5-flash",          "Gemini 2.5 Flash — 快速多模态 (1M token)"),
    ("gemini-2.5-flash-lite",     "Gemini 2.5 Flash Lite — 最低成本"),
    // ── 预览版 ──
    ("gemini-3-pro-preview",      "Gemini 3 Pro Preview — SOTA 推理 + Agent"),
    ("gemini-3-flash-preview",    "Gemini 3 Flash Preview — 最快智能模型"),
    // ── 别名 ──
    ("gemini-pro-latest",         "Gemini Pro Latest (→ 2.5-pro alias)"),
    ("gemini-flash-latest",       "Gemini Flash Latest (→ 2.5-flash alias)"),
    ("gemini-flash-lite-latest",  "Gemini Flash-Lite Latest (→ 2.5-flash-lite alias)"),
    // ── 图像生成 ──
    ("gemini-2.5-flash-image",    "Gemini 2.5 Flash Image — 图像生成/编辑"),
    // ── 旧版 (供对照) ──
    ("gemini-2.0-flash",          "Gemini 2.0 Flash (旧版)"),
    ("gemini-1.5-pro",            "Gemini 1.5 Pro (旧版)"),
];

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let token_path = std::env::var("ANTIGRAVITY_API_KEY").ok()
        .or_else(|| args.get(1).cloned())
        .unwrap_or_else(|| "dummy_credential".to_string());

    // 1. 从缓存文件读取 access_token
    let token_data: serde_json::Value = {
        let content = std::fs::read_to_string(&token_path)
            .map_err(|e| anyhow::anyhow!(
                "无法读取 token 文件 '{}': {}\n提示：请先运行 antigravity_chat 或 antigravity_auth 完成登录",
                token_path, e
            ))?;
        serde_json::from_str(&content)?
    };

    let access_token = token_data.get("access_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("token 文件中缺少 access_token 字段"))?;

    let project_id = token_data.get("extra")
        .and_then(|e| e.get("project_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let http = reqwest::Client::new();

    // ── Part 1: CloudCode PA 项目配置 ──
    println!("=== [1/2] CloudCode PA 项目配置 ===\n");
    {
        let url = "https://cloudcode-pa.googleapis.com/v1internal:loadCodeAssist";
        let body = serde_json::json!({
            "metadata": {
                "ideType": "ANTIGRAVITY",
                "platform": "PLATFORM_UNSPECIFIED",
                "pluginType": "GEMINI"
            }
        });

        let res = http.post(url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .header("User-Agent", "google-api-nodejs-client/9.15.1")
            .header("X-Goog-Api-Client", "gl-python/3.12.0")
            .header("Client-Metadata", r#"{"ideType":"IDE_UNSPECIFIED","platform":"PLATFORM_UNSPECIFIED","pluginType":"GEMINI"}"#)
            .json(&body)
            .send()
            .await?;

        let status = res.status();
        let text = res.text().await?;

        if !status.is_success() {
            eprintln!("loadCodeAssist 失败 ({}): {}", status, text);
        } else {
            let json: serde_json::Value = serde_json::from_str(&text)?;

            if let Some(project) = json.get("cloudaicompanionProject") {
                if let Some(p) = project.as_str() {
                    println!("项目 ID: {}", p);
                } else if let Some(obj) = project.as_object() {
                    if let Some(id) = obj.get("id").and_then(|v| v.as_str()) {
                        println!("项目 ID: {}", id);
                    }
                }
            }

            if let Some(tier) = json.get("currentTier") {
                let id = tier.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
                let name = tier.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
                println!("当前 Tier: {} ({})", name, id);
            }

            if let Some(paid) = json.get("paidTier") {
                let name = paid.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
                if let Some(credits) = paid.get("availableCredits").and_then(|v| v.as_array()) {
                    for c in credits {
                        let amount = c.get("creditAmount").and_then(|v| v.as_str()).unwrap_or("?");
                        let ctype = c.get("creditType").and_then(|v| v.as_str()).unwrap_or("?");
                        println!("付费 Tier: {} (额度: {} {})", name, amount, ctype);
                    }
                }
            }
        }
    }

    // ── Part 2: 模型可用性探测 ──
    println!("\n=== [2/2] CloudCode PA 模型可用性 ===");
    println!("(基于 CLIProxyAPI GetGeminiCLIModels 标准列表)\n");

    let base_url = "https://cloudcode-pa.googleapis.com/v1internal";

    for (model_name, description) in CANDIDATE_MODELS {
        let probe_body = serde_json::json!({
            "model": model_name,
            "userAgent": "antigravity",
            "requestType": "agent",
            "project": project_id,
            "requestId": format!("probe-{}", Uuid::new_v4()),
            "request": {
                "contents": [{
                    "role": "user",
                    "parts": [{"text": "hi"}]
                }],
                "generationConfig": {
                    "maxOutputTokens": 1
                }
            }
        });

        let res = http.post(format!("{}:generateContent", base_url))
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .header("User-Agent", "google-api-nodejs-client/9.15.1")
            .header("X-Goog-Api-Client", "gl-python/3.12.0")
            .header("Client-Metadata", r#"{"ideType":"IDE_UNSPECIFIED","platform":"PLATFORM_UNSPECIFIED","pluginType":"GEMINI"}"#)
            .json(&probe_body)
            .send()
            .await;

        match res {
            Ok(resp) => {
                let status = resp.status();
                let code = status.as_u16();
                let text = resp.text().await.unwrap_or_default();

                let (icon, reason) = if status.is_success() || code == 200 {
                    ("✅", "可用".to_string())
                } else if code == 429 {
                    ("✅", "可用 (配额限制中)".to_string())
                } else if code == 404 {
                    ("❌", "不存在".to_string())
                } else if code == 403 {
                    let msg = serde_json::from_str::<serde_json::Value>(&text)
                        .ok()
                        .and_then(|j| j.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str()).map(|s| s.to_string()))
                        .unwrap_or_else(|| format!("权限不足 ({})", code));
                    ("⚠️", msg)
                } else {
                    let msg = serde_json::from_str::<serde_json::Value>(&text)
                        .ok()
                        .and_then(|j| j.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str()).map(|s| s.to_string()))
                        .unwrap_or_else(|| format!("HTTP {}", code));
                    ("❓", msg)
                };

                println!("  {} {:<28} {}", icon, model_name, reason);
                if !reason.contains("不存在") {
                    println!("     └─ {}", description);
                }
            }
            Err(e) => {
                println!("  ❓ {:<28} 网络错误: {}", model_name, e);
            }
        }

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    println!();
    Ok(())
}
