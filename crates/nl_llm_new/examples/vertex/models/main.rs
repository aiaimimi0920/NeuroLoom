use anyhow::Result;
use nl_llm_new::auth::providers::vertex_sa::VertexSAAuth;
use serde_json::Value;

#[tokio::main]
async fn main() -> Result<()> {
    // 加载内容
    let sa_json = std::env::var("VERTEX_SA_JSON").unwrap_or_else(|_| "{}".to_string());
    let location = std::env::var("VERTEX_LOCATION").unwrap_or_else(|_| "us-central1".to_string());
    
    // 尝试直接读取作为文件路径，支持工作目录不同时的容错
    let mut sa_json_content = "{}".to_string();
    let cwd = std::env::current_dir().unwrap_or_default();
    
    // 候选路径列表
    let candidates = vec![
        std::path::PathBuf::from(&sa_json),
        cwd.join(&sa_json),
        cwd.join("crates/nl_llm_new").join(&sa_json),
        std::path::PathBuf::from("../../").join(&sa_json),
        std::path::PathBuf::from("../../crates/nl_llm_new").join(&sa_json),
    ];

    let mut found_path = None;
    for path in &candidates {
        if path.is_file() {
            match std::fs::read_to_string(path) {
                Ok(content) => {
                    if content.trim().starts_with('{') {
                        sa_json_content = content;
                        found_path = Some(path.clone());
                        break;
                    }
                }
                Err(_) => continue,
            }
        }
    }

    if found_path.is_none() {
        if sa_json.trim().starts_with('{') {
            sa_json_content = sa_json;
        } else {
            println!("警告: 未找到 SA JSON 文件并且内容不是 JSON 格式。\n当前工作目录: {}\n尝试寻找的路径为环境变量设定的: {}", cwd.display(), sa_json);
        }
    } else {
        println!("调试: 成功从 {} 读取了 SA JSON", found_path.unwrap().display());
    }

    println!("========================================");
    println!("  Vertex Models List Validation");
    println!("========================================");
    
        let mut auth = VertexSAAuth::new(sa_json_content);
    let _project = match auth.project_id() {
        Ok(project) => project,
        Err(e) => {
            println!("❌ 必须提供合法的 Service Account: {}", e);
            return Ok(());
        }
    };

    println!("正在通过 Service Account 换取令牌...");
    if let Err(e) = auth.ensure_authenticated().await {
        println!("❌ Token 获取失败: {}", e);
        return Ok(());
    }

    let token = auth.token().unwrap();
    let url = format!(
        "https://{}-aiplatform.googleapis.com/v1beta1/publishers/google/models",
        location
    );

    println!("请求 URL: {}", url);
    
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;
        
    let status = resp.status();
    let body = resp.text().await?;

    if !status.is_success() {
        println!("❌ 获取模型列表失败 (响应码 {})", status);
        println!("{}", body);
        return Ok(());
    }

    let json: Value = serde_json::from_str(&body)?;
    let mut models = Vec::new();
    
    if let Some(arr) = json.get("publisherModels").and_then(|m| m.as_array()) {
        for m in arr {
            if let Some(name) = m.get("name").and_then(|n| n.as_str()) {
                if name.contains("gemini") {
                    // Extract just the model name part
                    let short_name = name.split('/').last().unwrap_or(name);
                    models.push(short_name.to_string());
                }
            }
        }
    }

    println!("\n✅ 解析成功! 可用 Gemini 模型列表 (总数: {}):\n", models.len());
    
    for (i, m) in models.iter().enumerate() {
        println!("  {}. {}", i + 1, m);
    }

    println!("========================================");
    Ok(())
}
