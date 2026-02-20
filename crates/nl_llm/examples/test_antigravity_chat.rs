//! Antigravity (Gemini Code Assist) 对话测试
//!
//! 用法:
//!   test_antigravity_chat.exe [prompt] [--stream]
//!
//! 示例:
//!   test_antigravity_chat.exe "用 Rust 写个 Hello World"
//!   test_antigravity_chat.exe "解释一下 async/await" --stream

use nl_llm::prompt_ast::{PromptAst, PromptNode};
use nl_llm::provider::antigravity::AntigravityProvider;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // 解析参数
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
    println!("  模式: {}", if use_stream { "流式 (streamGenerateContent)" } else { "非流式 (generateContent)" });
    println!("========================================");
    println!();
    println!("用户: {}", prompt);
    println!();

    // 使用 builder 模式构建 PromptAst（nodes 字段是私有的）
    let ast = PromptAst::new().push(PromptNode::User(prompt.clone()));

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        run_chat(ast, use_stream).await;
    });
}

async fn run_chat(ast: nl_llm::prompt_ast::PromptAst, use_stream: bool) {
    // 使用默认配置创建 Provider（自动读取已保存的 token）
    let provider = AntigravityProvider::default_provider();

    print!("正在验证身份... ");
    match provider.ensure_authenticated().await {
        Ok(token) => {
            println!("✓ (token: {}...)", &token[..20.min(token.len())]);
        }
        Err(e) => {
            println!("✗");
            eprintln!("认证失败: {:?}", e);
            eprintln!();
            eprintln!("提示: 请先运行 test_antigravity.bat 完成首次登录");
            std::process::exit(1);
        }
    }
    println!();

    println!("正在请求模型...");
    println!();

    let result = if use_stream {
        provider.stream_complete(&ast).await
    } else {
        provider.complete(&ast).await
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
            eprintln!("  1. 检查账户是否有 Gemini Code Assist 订阅");
            eprintln!("  2. 如果 token 已过期，删除 %USERPROFILE%\\.nl_llm\\antigravity_token.json 后重新登录");
            eprintln!("  3. 检查是否能访问 cloudcode-pa.googleapis.com");
            std::process::exit(1);
        }
    }
}
