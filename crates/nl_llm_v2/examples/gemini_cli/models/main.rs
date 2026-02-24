//! gemini_cli 平台测试 - models
//!
//! 由于 Gemini CLI 的 OAuth `client_id` 并未获得 `v1internal:fetchAvailableModels` 的相关 API 权限，
//! 此实例展示的是从我们拓展模块里面退货的内置后备（静态）可用模型列表。
//!
//! 运行方式: cargo run --example gemini_cli_models
//! 或直接运行: test.bat

use anyhow::Result;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let cache_path: PathBuf = std::env::var("GEMINI_CLI_CACHE_PATH").ok()
        .or_else(|| args.get(1).cloned())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            path.push("examples");
            path.push("gemini_cli");
            path.push(".cache");
            path.push("oauth_token.json");
            path
        });

    println!("缓存文件: {}\n", cache_path.display());

    let client = nl_llm_v2::LlmClient::from_preset("gemini_cli")
        .expect("Preset should exist")
        .with_gemini_cli_oauth(&cache_path)
        .build();

    println!("=== Gemini CLI 可用模型列表 ===");
    println!("(内建静态回退列表：基于 GeminiCliExtension)\n");

    let models_list = client.list_models().await?;

    for (index, m) in models_list.iter().enumerate() {
        if !m.description.is_empty() {
            println!("  {}. {:<32} {}", index + 1, m.id, m.description);
        } else {
            println!("  {}. {}", index + 1, m.id);
        }
    }

    println!("\n共计 {} 个模型", models_list.len());

    Ok(())
}
