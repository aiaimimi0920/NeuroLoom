//! Google AI Studio 对话测试
//!
//! 通过 API Key 访问 Google AI Studio (generativelanguage.googleapis.com)
//!
//! 用法:
//!   test_google_ai_studio_chat.exe [prompt] [--stream] --key API_KEY [--model MODEL]
//!
//! 示例:
//!   test_google_ai_studio_chat.exe "你好" --key AIzaSy...
//!   test_google_ai_studio_chat.exe "解释 Rust 生命周期" --key AIzaSy... --stream --model gemini-2.5-pro
//!
//! 环境变量（可替代命令行参数）:
//!   GOOGLE_AI_STUDIO_API_KEY  - API Key（从 https://aistudio.google.com/app/apikey 获取）
//!   GOOGLE_AI_STUDIO_MODEL    - 模型名称（默认 gemini-2.5-flash）

use nl_llm::prompt_ast::{PromptAst, PromptNode};
use nl_llm::provider::google_ai_studio::{GoogleAIStudioConfig, GoogleAIStudioProvider};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // ── 解析命令行参数 ────────────────────────────────────────────────────────────
    let use_stream = args.iter().any(|a| a == "--stream");

    // --key <api_key>
    let api_key = args
        .windows(2)
        .find(|w| w[0] == "--key")
        .map(|w| w[1].clone())
        .or_else(|| std::env::var("GOOGLE_AI_STUDIO_API_KEY").ok());

    // --model <model>
    let model = args
        .windows(2)
        .find(|w| w[0] == "--model")
        .map(|w| w[1].clone())
        .or_else(|| std::env::var("GOOGLE_AI_STUDIO_MODEL").ok())
        .unwrap_or_else(|| "gemini-2.5-flash".to_string());

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
            if *a == "--key" || *a == "--model" {
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
    println!("  Google AI Studio Chat");
    println!("========================================");

    let api_key = match api_key {
        Some(key) => {
            println!("  认证: API Key ({}...)", &key[..8.min(key.len())]);
            key
        }
        None => {
            eprintln!("[错误] 未提供 API Key！");
            eprintln!();
            eprintln!("请通过以下方式之一提供认证：");
            eprintln!("  命令行: --key AIzaSy...");
            eprintln!("  环境变量: GOOGLE_AI_STUDIO_API_KEY=...");
            eprintln!();
            eprintln!("获取 API Key: https://aistudio.google.com/app/apikey");
            std::process::exit(1);
        }
    };

    println!("  模型: {}", model);
    println!("  Base URL: https://generativelanguage.googleapis.com");
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
    let config = GoogleAIStudioConfig { api_key, model };
    let provider = GoogleAIStudioProvider::new(config);

    // ── 构建 AST 并发起请求 ──────────────────────────────────────────────────────
    let ast = PromptAst::new().push(PromptNode::User(prompt));

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        run_chat(&provider, &ast, use_stream).await;
    });
}

async fn run_chat(provider: &GoogleAIStudioProvider, ast: &PromptAst, use_stream: bool) {
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
            eprintln!("  1. 检查 API Key 是否有效");
            eprintln!("  2. 确认已启用 Generative Language API");
            eprintln!("  3. 检查模型名称是否正确");
            std::process::exit(1);
        }
    }
}
