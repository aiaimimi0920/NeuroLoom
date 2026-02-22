//! iFlow 对话测试
//!
//! 通过 Cookie → API Key → OpenAI-compat 接口
//!
//! 配置文件: examples/iflow/iflow_config.txt
//! 或命令行: --cookie BXAuth=...

use nl_llm_new::primitive::PrimitiveRequest;
use nl_llm_new::provider::iflow::config::IFlowConfig;
use nl_llm_new::provider::iflow::provider::IFlowProvider;
use nl_llm_new::provider::LlmProvider;
use futures::StreamExt;
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let use_stream = args.iter().any(|a| a == "--stream");

    // 读取配置
    let config_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("iflow")
        .join("iflow_config.txt");

    let (cookie_from_config, default_prompt, default_model) = read_config(&config_path);

    let cookie = args.windows(2).find(|w| w[0] == "--cookie").map(|w| w[1].clone())
        .or_else(|| std::env::var("IFLOW_COOKIE").ok())
        .or(cookie_from_config);

    let model = args.windows(2).find(|w| w[0] == "--model").map(|w| w[1].clone())
        .or_else(|| std::env::var("IFLOW_MODEL").ok())
        .unwrap_or(default_model);

    let prompt = args.iter().skip(1)
        .filter(|a| !a.starts_with("--") && {
            // skip args that are values of --cookie or --model
            let idx = args.iter().position(|x| x == *a).unwrap();
            idx == 0 || (args[idx-1] != "--cookie" && args[idx-1] != "--model")
        })
        .cloned()
        .next()
        .unwrap_or(default_prompt);

    println!("========================================");
    println!("  iFlow Chat (nl_llm_new)");
    println!("========================================");

    let cookie = match cookie {
        Some(c) => {
            println!("  认证: Cookie (前20字符: {}...)", &c[..20.min(c.len())]);
            c
        }
        None => {
            eprintln!("[错误] 未提供 Cookie！");
            eprintln!("  命令行: --cookie BXAuth=...");
            eprintln!("  环境变量: IFLOW_COOKIE=BXAuth=...");
            eprintln!("  配置文件: examples/iflow/iflow_config.txt");
            std::process::exit(1);
        }
    };

    println!("  模型: {}", model);
    println!("  模式: {}", if use_stream { "流式" } else { "非流式" });
    println!("========================================");
    println!();
    println!("用户: {}", prompt);
    println!();

    let token_path = dirs::cache_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("neuroloom")
        .join("iflow_token.json");

    let provider = IFlowProvider::new(IFlowConfig::new(cookie, model, token_path));

    let primitive = PrimitiveRequest::single_user_message(&prompt);
    let body = provider.compile(&primitive);

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        if use_stream {
            println!("正在请求模型 (流式)...");
            match provider.stream(body).await {
                Ok(mut stream) => {
                    print!("AI: ");
                    while let Some(chunk) = stream.next().await {
                        match chunk {
                            Ok(c) => {
                                if let nl_llm_new::provider::ChunkDelta::Text(t) = &c.delta {
                                    print!("{}", t);
                                }
                            }
                            Err(e) => { eprintln!("\n流式错误: {:?}", e); break; }
                        }
                    }
                    println!();
                }
                Err(e) => eprintln!("请求失败: {:?}", e),
            }
        } else {
            println!("正在请求模型...");
            match provider.complete(body).await {
                Ok(response) => {
                    println!("----------------------------------------");
                    println!("AI 回复:");
                    println!("----------------------------------------");
                    println!("{}", response.content);
                    println!("----------------------------------------");
                }
                Err(e) => { eprintln!("请求失败: {:?}", e); std::process::exit(1); }
            }
        }
    });
}

fn read_config(path: &Path) -> (Option<String>, String, String) {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return (None, "Hello".to_string(), "qwen3-max".to_string()),
    };

    let mut token = None;
    let mut prompt = "Hello".to_string();
    let mut model = "qwen3-max".to_string();

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() { continue; }
        if line.starts_with("BXAuth=") {
            token = Some(line.to_string());
        } else if line.starts_with("MODEL=") {
            model = line[6..].to_string();
        } else if line.starts_with("PROMPT=") {
            prompt = line[7..].to_string();
        }
    }

    (token, prompt, model)
}
