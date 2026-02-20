//! Gemini CLI 交互式聊天测试
//!
//! 用法: cargo run --example test_gemini_cli_chat -p nl_llm

use nl_llm::prompt_ast::{PromptAst, PromptNode};
use nl_llm::provider::gemini_cli::GeminiCliProvider;

#[tokio::main]
async fn main() {
    // 从命令行参数判断模式
    let args: Vec<String> = std::env::args().collect();
    let use_stream = args.iter().any(|a| a == "--stream" || a == "-s");

    let ast = PromptAst::new()
        .push(PromptNode::System(
            "你是一个有用的AI助手。请用中文回答。".to_string(),
        ))
        .push(PromptNode::User("你好，请介绍一下你自己。".to_string()));

    if use_stream {
        println!("Mode: Streaming");
    } else {
        println!("Mode: Non-streaming");
    }
    println!("----------------------------------------");

    run_chat(ast, use_stream).await;
}

async fn run_chat(ast: nl_llm::prompt_ast::PromptAst, use_stream: bool) {
    let provider = GeminiCliProvider::default_provider();

    print!("正在验证身份... ");
    match provider.ensure_authenticated().await {
        Ok(token) => {
            println!(
                "✓ (token: {}...)",
                &token[..20.min(token.len())]
            );
        }
        Err(e) => {
            println!("✗");
            eprintln!("认证失败: {:?}", e);
            return;
        }
    }

    println!("正在请求模型...");

    let result = if use_stream {
        provider.stream_complete(&ast).await
    } else {
        provider.complete(&ast).await
    };

    match result {
        Ok(text) => {
            println!("{}", text);
        }
        Err(e) => {
            eprintln!("请求失败: {:?}", e);
        }
    }

    println!("----------------------------------------");
}
