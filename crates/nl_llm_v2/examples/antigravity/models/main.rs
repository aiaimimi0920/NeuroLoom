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
use std::path::PathBuf;

/// 已知模型描述 (用于在 fetchAvailableModels 结果中注释模型)
/// 模型名使用 CloudCode PA 的内部名称
/// 参照: CLIProxyAPI GetAntigravityModelConfig() + defaultAntigravityAliases()
const CANDIDATE_MODELS: &[(&str, &str)] = &[
    // ═══ Gemini 系列 (内部名称) ═══
    ("gemini-2.5-pro",            "Gemini 2.5 Pro — 最强推理 (1M token)"),
    ("gemini-2.5-flash",          "Gemini 2.5 Flash — 快速多模态 (1M token)"),
    ("gemini-2.5-flash-lite",     "Gemini 2.5 Flash Lite — 最低成本"),
    ("gemini-3-pro-high",         "Gemini 3 Pro (alias: gemini-3-pro-preview)"),
    ("gemini-3.1-pro-high",       "Gemini 3.1 Pro (最新)"),
    ("gemini-3.1-pro-low",        "Gemini 3.1 Pro (低 Thinking)"),
    ("gemini-3-flash",            "Gemini 3 Flash (alias: gemini-3-flash-preview)"),
    ("gemini-3-pro-image",        "Gemini 3 Pro Image (alias: gemini-3-pro-image-preview)"),

    // ═══ Claude 系列 (via CloudCode PA) ═══
    ("claude-opus-4-6-thinking",   "Claude Opus 4.6 + Thinking — 1M ctx"),
    ("claude-sonnet-4-6",          "Claude Sonnet 4.6 — 200K ctx"),
    ("claude-sonnet-4-6-thinking", "Claude Sonnet 4.6 + Thinking"),
    ("claude-opus-4-5-thinking",   "Claude Opus 4.5 + Thinking"),
    ("claude-sonnet-4-5",          "Claude Sonnet 4.5"),
    ("claude-sonnet-4-5-thinking", "Claude Sonnet 4.5 + Thinking"),

    // ═══ 其他 ═══
    ("gpt-oss-120b-medium",        "GPT OSS 120B Medium"),
];

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // [修复] 使用编译期 CARGO_MANIFEST_DIR 定位缓存路径
    // current_exe() 不可靠：CARGO_TARGET_DIR 可能设在 temp 目录
    let cache_path: PathBuf = std::env::var("ANTIGRAVITY_CACHE_PATH").ok()
        .or_else(|| args.get(1).cloned())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            path.push("examples");
            path.push("antigravity");
            path.push(".cache");
            path.push("oauth_token.json");
            path
        });

    println!("缓存文件: {}\n", cache_path.display());

    // 1. 从缓存文件读取 access_token
    let token_data: serde_json::Value = {
        let content = std::fs::read_to_string(&cache_path)
            .map_err(|e| anyhow::anyhow!(
                "无法读取 token 文件 '{}': {}\n提示：请先运行 antigravity_chat 或 antigravity_auth 完成登录",
                cache_path.display(), e
            ))?;
        serde_json::from_str(&content)?
    };

    let access_token = token_data.get("access_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("token 文件中缺少 access_token 字段"))?;

    let _project_id = token_data.get("extra")
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

    // ── Part 2: 通过 fetchAvailableModels 获取所有可用模型 ──
    println!("\n=== [2/2] CloudCode PA 可用模型列表 ===");
    println!("(via /v1internal:fetchAvailableModels)\n");

    let base_url = "https://cloudcode-pa.googleapis.com/v1internal";
    let fetch_url = format!("{}:fetchAvailableModels", base_url);

    let res = http.post(&fetch_url)
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Content-Type", "application/json")
        .header("User-Agent", "antigravity/1.104.0 darwin/arm64")
        .json(&serde_json::json!({}))
        .send()
        .await?;

    let status = res.status();
    let text = res.text().await?;

    if !status.is_success() {
        eprintln!("fetchAvailableModels 失败 ({}): {}", status, text);
        return Ok(());
    }

    let json: serde_json::Value = serde_json::from_str(&text)?;

    // 已知模型的描述 (用于注释)
    let known_descriptions: std::collections::HashMap<&str, &str> = CANDIDATE_MODELS.iter()
        .map(|(name, desc)| (*name, *desc))
        .collect();

    // 内部模型过滤 (参照 FetchAntigravityModels 中的过滤)
    let skip_models = ["chat_20706", "chat_23310", "gemini-2.5-flash-thinking", "gemini-3-pro-low"];

    if let Some(models) = json.get("models").and_then(|v| v.as_object()) {
        let mut gemini_models: Vec<(&str, &serde_json::Value)> = Vec::new();
        let mut claude_models: Vec<(&str, &serde_json::Value)> = Vec::new();
        let mut other_models: Vec<(&str, &serde_json::Value)> = Vec::new();

        for (name, data) in models {
            let name_str = name.as_str();
            if skip_models.contains(&name_str) {
                continue;
            }
            if name_str.starts_with("claude") {
                claude_models.push((name_str, data));
            } else if name_str.starts_with("gemini") {
                gemini_models.push((name_str, data));
            } else {
                other_models.push((name_str, data));
            }
        }

        gemini_models.sort_by_key(|&(name, _)| name);
        claude_models.sort_by_key(|&(name, _)| name);
        other_models.sort_by_key(|&(name, _)| name);

        let print_model = |name: &str, _data: &serde_json::Value| {
            if let Some(desc) = known_descriptions.get(name) {
                println!("  ✅ {:<32} {}", name, desc);
            } else {
                println!("  ✅ {}", name);
            }
        };

        if !gemini_models.is_empty() {
            println!("── Gemini ({} 个) ──", gemini_models.len());
            for (name, data) in &gemini_models {
                print_model(name, data);
            }
            println!();
        }

        if !claude_models.is_empty() {
            println!("── Claude ({} 个) ──", claude_models.len());
            for (name, data) in &claude_models {
                print_model(name, data);
            }
            println!();
        }

        if !other_models.is_empty() {
            println!("── 其他 ({} 个) ──", other_models.len());
            for (name, data) in &other_models {
                print_model(name, data);
            }
            println!();
        }

        let total = gemini_models.len() + claude_models.len() + other_models.len();
        println!("共计 {} 个可用模型", total);
    } else {
        eprintln!("fetchAvailableModels 返回数据中无 models 字段");
        eprintln!("原始响应: {}", &text[..text.len().min(500)]);
    }

    println!();
    Ok(())
}

