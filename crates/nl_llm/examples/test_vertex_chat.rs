//! Vertex AI (Google Cloud Gemini) 对话测试
//!
//! 支持两种认证模式：
//!   1. Service Account JSON 文件（推荐）
//!   2. API Key
//!
//! 用法:
//!   test_vertex_chat.exe [prompt] [--stream] [--sa path/to/sa.json] [--key AIza...]
//!
//! 示例:
//!   test_vertex_chat.exe "你好" --sa C:\my-sa.json
//!   test_vertex_chat.exe "解释 Rust 生命周期" --key AIzaSy... --stream
//!
//! 环境变量（可替代命令行参数）:
//!   VERTEX_SA_JSON_PATH   - Service Account JSON 文件路径
//!   VERTEX_API_KEY        - Google API Key
//!   VERTEX_MODEL          - 模型名称（默认 gemini-2.0-flash）
//!   VERTEX_LOCATION       - 区域（默认 us-central1）

use nl_llm::prompt_ast::{PromptAst, PromptNode};
use nl_llm::provider::vertex::{VertexConfig, VertexProvider};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // ── 解析命令行参数 ────────────────────────────────────────────────────────────
    let use_stream = args.iter().any(|a| a == "--stream");

    // --sa <path>
    let sa_path = args
        .windows(2)
        .find(|w| w[0] == "--sa")
        .map(|w| w[1].clone())
        .or_else(|| std::env::var("VERTEX_SA_JSON_PATH").ok());

    // --key <api_key>
    let api_key = args
        .windows(2)
        .find(|w| w[0] == "--key")
        .map(|w| w[1].clone())
        .or_else(|| std::env::var("VERTEX_API_KEY").ok());

    // --model <model>
    let model = args
        .windows(2)
        .find(|w| w[0] == "--model")
        .map(|w| w[1].clone())
        .or_else(|| std::env::var("VERTEX_MODEL").ok())
        .unwrap_or_else(|| "gemini-2.0-flash".to_string());

    // --location <loc>
    let location = args
        .windows(2)
        .find(|w| w[0] == "--location")
        .map(|w| w[1].clone())
        .or_else(|| std::env::var("VERTEX_LOCATION").ok());

    // prompt（跳过所有 -- 选项和其值）
    let mut skip_next = false;
    let prompt = args
        .iter()
        .skip(1)
        .filter(|a| {
            if skip_next {
                skip_next = false;
                return false;
            }
            if *a == "--sa" || *a == "--key" || *a == "--model" || *a == "--location" {
                skip_next = true;
                return false;
            }
            !a.starts_with("--")
        })
        .cloned()
        .next()
        .unwrap_or_else(|| "你好！请用中文简单介绍一下你自己，以及你能做什么？".to_string());

    // ── 打印配置概览 ──────────────────────────────────────────────────────────────
    println!("========================================");
    println!("  Vertex AI (Google Cloud Gemini) Chat");
    println!("========================================");

    let auth_mode = if sa_path.is_some() {
        format!("Service Account JSON ({})", sa_path.as_deref().unwrap_or(""))
    } else if api_key.is_some() {
        "API Key".to_string()
    } else {
        eprintln!("[错误] 未提供认证信息！");
        eprintln!();
        eprintln!("请通过以下方式之一提供认证：");
        eprintln!("  命令行: --sa path/to/sa.json");
        eprintln!("    或者: --key AIzaSy...");
        eprintln!();
        eprintln!("  环境变量: VERTEX_SA_JSON_PATH=...");
        eprintln!("    或者:   VERTEX_API_KEY=...");
        eprintln!();
        eprintln!("详见 examples\\vertex_config.txt");
        std::process::exit(1);
    };

    println!("  认证: {}", auth_mode);
    println!("  模型: {}", model);
    println!(
        "  区域: {}",
        location.as_deref().unwrap_or("us-central1 (默认)")
    );
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

    // ── 构建 Provider ────────────────────────────────────────────────────────────
    let sa_json = sa_path.map(|p| {
        std::fs::read_to_string(&p).unwrap_or_else(|e| {
            eprintln!("[错误] 无法读取 SA JSON 文件 {}: {}", p, e);
            std::process::exit(1);
        })
    });

    let config = VertexConfig {
        service_account_json: sa_json,
        api_key,
        location,
        model,
    };
    let provider = VertexProvider::new(config);

    // ── 构建 AST 并发起请求 ──────────────────────────────────────────────────────
    let ast = PromptAst::new().push(PromptNode::User(prompt));

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        run_chat(&provider, &ast, use_stream).await;
    });
}

async fn run_chat(provider: &VertexProvider, ast: &PromptAst, use_stream: bool) {
    println!("正在请求模型...");
    println!();

    let result = if use_stream {
        provider.stream_complete(ast).await
    } else {
        provider.complete(ast).await
    };

    match result {
        Ok(response) => {
            println!("----------------------------------------");
            println!("AI 回复:");
            println!("----------------------------------------");
            println!("{}", response);
            println!("----------------------------------------");
        }
        Err(e) => {
            eprintln!();
            eprintln!("========================================");
            eprintln!("  请求失败");
            eprintln!("========================================");
            eprintln!("{:?}", e);
            eprintln!();
            eprintln!("排查建议:");
            eprintln!("  1. 检查 SA JSON 文件是否有效，project_id / private_key 是否正确");
            eprintln!("  2. 检查 Service Account 是否有 Vertex AI 访问权限");
            eprintln!("     （需要 roles/aiplatform.user 或更高权限）");
            eprintln!("  3. 如使用 API Key，确认已在 GCP Console 中启用 Vertex AI API");
            eprintln!("  4. 检查模型名称和区域是否匹配");
            std::process::exit(1);
        }
    }
}
