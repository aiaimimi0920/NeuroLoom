use nl_llm_new::primitive::{PrimitiveMessage, PrimitiveRequest};
use nl_llm_new::provider::gemini_cli::{GeminiCliConfig, GeminiCliProvider};
use nl_llm_new::provider::LlmProvider;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let use_stream = args.iter().any(|a| a == "--stream");
    let prompt = args
        .iter()
        .skip(1)
        .find(|a| !a.starts_with("--"))
        .cloned()
        .unwrap_or_else(|| "你好！请用中文简单介绍一下你自己。".to_string());

    let mut config = GeminiCliConfig::default();
    config.model = "gemini-2.5-flash".to_string();

    println!("========================================");
    println!("  Gemini CLI Chat Test (nl_llm_new) [Provider API]");
    println!("========================================");
    println!(
        "  模式: {}",
        if use_stream {
            "流式 (streamGenerateContent)"
        } else {
            "非流式 (generateContent)"
        }
    );
    println!("  模型: {}", config.model);
    println!("========================================");
    println!();
    println!("用户: {}", prompt);
    println!();

    print!("正在初始化并验证身份... ");
    let mut provider = match GeminiCliProvider::new(config) {
        Ok(p) => p,
        Err(e) => {
            println!("✗");
            eprintln!("初始化 Provider 失败: {:?}", e);
            std::process::exit(1);
        }
    };
    
    if provider.needs_refresh() {
        if let Err(e) = provider.refresh_auth().await {
            println!("✗");
            eprintln!("Token 刷新失败: {:?}", e);
            std::process::exit(1);
        }
    }
    println!("✓ 已就绪");
    println!();

    println!("正在请求模型...");
    println!();

    let request = PrimitiveRequest {
        model: "".to_string(), // 使用 config 中的默认模型
        messages: vec![PrimitiveMessage::user(prompt.clone())],
        system: None,
        tools: Vec::new(),
        ..Default::default()
    };

    let payload = provider.compile(&request);

    if use_stream {
        match provider.stream(payload).await {
            Ok(mut stream) => {
                while let Some(chunk_res) = stream.next().await {
                    match chunk_res {
                        Ok(chunk) => {
                            if let nl_llm_new::provider::ChunkDelta::Text(text) = chunk.delta {
                                print!("{}", text);
                                use std::io::Write;
                                std::io::stdout().flush().unwrap();
                            }
                        }
                        Err(e) => {
                            eprintln!("\n读取流数据时发生错误: {:?}", e);
                            break;
                        }
                    }
                }
                println!();
                println!("----------------------------------------");
            }
            Err(e) => {
                eprintln!();
                eprintln!("请求失败: {:?}", e);
                std::process::exit(1);
            }
        }
    } else {
        match provider.complete(payload).await {
            Ok(reply) => {
                println!("----------------------------------------");
                println!("AI 回复:");
                println!("----------------------------------------");
                println!("{}", reply.content);
                println!("----------------------------------------");
            }
            Err(e) => {
                eprintln!();
                eprintln!("请求失败: {:?}", e);
                std::process::exit(1);
            }
        }
    }
    
    Ok(())
}
