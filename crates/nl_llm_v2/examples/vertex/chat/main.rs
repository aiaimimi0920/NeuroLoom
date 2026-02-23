//! vertex 平台测试 - chat
//!
//! 运行方式: cargo run --example vertex_chat
//! 或直接运行: test.bat
//!
//! [修复] 需要提供 project_id 和 location，或从 SA JSON 中提取

use nl_llm_v2::{LlmClient, PrimitiveRequest, VertexSite, GeminiProtocol};
use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ServiceAccountInfo {
    project_id: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let sa_json = std::env::var("GOOGLE_APPLICATION_CREDENTIALS_JSON").ok()
        .or_else(|| args.get(1).cloned())
        .unwrap_or_else(|| "dummy_credential".to_string());

    // [新增] 从 SA JSON 中提取 project_id
    let project_id = extract_project_id(&sa_json).unwrap_or_else(|| "PLACEHOLDER_PROJECT_ID".to_string());
    let location = args.get(3).cloned().unwrap_or_else(|| "us-central1".to_string());

    // [修复] 使用正��的 project_id 和 location 构建 VertexSite
    let client = LlmClient::builder()
        .site(VertexSite::new(&project_id, &location))
        .protocol(GeminiProtocol {})
        .default_model("gemini-2.5-flash")
        .build();

    let prompt = args.get(2).cloned()
        .unwrap_or_else(|| "Hello!".to_string());

    let req = PrimitiveRequest::single_user_message(&prompt)
        .with_model("gemini-2.5-flash");

    println!("用户: {}\n", prompt);
    println!("AI:");

    let resp = client.complete(&req).await?;
    println!("{}", resp.content);

    Ok(())
}

/// 从 Service Account JSON 中提取 project_id
fn extract_project_id(json_str: &str) -> Option<String> {
    serde_json::from_str::<ServiceAccountInfo>(json_str)
        .ok()
        .map(|sa| sa.project_id)
}
