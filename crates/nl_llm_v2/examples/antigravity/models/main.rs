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

    let http = reqwest::Client::new();

    let client = nl_llm_v2::LlmClient::from_preset("antigravity")
        .expect("antigravity preset should exist")
        .with_antigravity_oauth(&cache_path)
        .build();

    // ── Part 1: CloudCode PA 项目配置 ──
    println!("=== [1/2] CloudCode PA 项目配置 ===\n");
    if let Ok(Some(balance_info)) = client.get_balance().await {
        println!("{}", balance_info);
    } else {
        println!("无法获取项目额度信息或该平台不支持。\n");
    }

    // ── Part 2: 通过 Extension API 获取所有可用模型 ──
    println!("\n=== [2/2] CloudCode PA 可用模型列表 ===");
    println!("(via Extension API)\n");

    let models_list = client.list_models().await?;

    let mut gemini_models = Vec::new();
    let mut claude_models = Vec::new();
    let mut other_models = Vec::new();

    for m in models_list {
        if m.id.starts_with("claude") {
            claude_models.push(m);
        } else if m.id.starts_with("gemini") {
            gemini_models.push(m);
        } else {
            other_models.push(m);
        }
    }

    let print_model = |m: &nl_llm_v2::provider::extension::ModelInfo| {
        if !m.description.is_empty() {
            println!("  ✅ {:<32} {}", m.id, m.description);
        } else {
            println!("  ✅ {}", m.id);
        }
    };

    if !gemini_models.is_empty() {
        println!("── Gemini ({} 个) ──", gemini_models.len());
        for m in &gemini_models { print_model(m); }
        println!();
    }

    if !claude_models.is_empty() {
        println!("── Claude ({} 个) ──", claude_models.len());
        for m in &claude_models { print_model(m); }
        println!();
    }

    if !other_models.is_empty() {
        println!("── 其他 ({} 个) ──", other_models.len());
        for m in &other_models { print_model(m); }
        println!();
    }

    let total = gemini_models.len() + claude_models.len() + other_models.len();
    println!("共计 {} 个可用模型\n", total);
    Ok(())
}

