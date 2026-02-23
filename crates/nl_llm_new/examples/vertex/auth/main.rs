use anyhow::Result;
use nl_llm_new::auth::providers::vertex_sa::VertexSAAuth;
use nl_llm_new::auth::TokenStatus;

#[tokio::main]
async fn main() -> Result<()> {
    // 加载内容
    let sa_json = std::env::var("VERTEX_SA_JSON").unwrap_or_else(|_| "{}".to_string());
    
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
    println!("  Vertex Service Account Validation");
    println!("========================================");
    
    let mut auth = VertexSAAuth::new(sa_json_content);

    println!("[1] 解析 SA JSON");
    match auth.project_id() {
        Ok(project) => {
            println!("  ✅ 成功。Project ID: {}", project);
        }
        Err(e) => {
            println!("  ❌ 失败。请检查 JSON 格式: {}", e);
            return Ok(());
        }
    }

    println!("\n[2] 开始验证 JWT 交互获取 Access Token...");
    match auth.ensure_authenticated().await {
        Ok(()) => {
            println!("  ✅ Token 成功获取！");
            if let Some(token) = auth.token() {
                println!("  🔑 Token: {}...", &token[..std::cmp::min(token.len(), 25)]);
            }

            match auth.token_status() {
                TokenStatus::Valid => {
                    println!("  ⏳ 过期状态: 有效");
                }
                _ => println!("  ⏳ 过期状态: 未知异常"),
            }
        }
        Err(e) => {
            println!("  ❌ Token 兑换失败: {}", e);
        }
    }

    println!("========================================");
    Ok(())
}
