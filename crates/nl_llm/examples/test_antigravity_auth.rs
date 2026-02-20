//! Antigravity (Gemini Code Assist) 认证测试示例
//!
//! 初次运行时会自动打开浏览器进行 OAuth2 登录，
//! 认证成功后 Token 会保存到 ~/.nl_llm/antigravity_token.json

use nl_llm::provider::antigravity::{AntigravityConfig, AntigravityProvider};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // 允许通过命令行指定 Token 文件路径（可选）
    let token_path = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".nl_llm").join("antigravity_token.json")
    };

    println!("===================================================");
    println!(" NeuroLoom - Antigravity (Gemini Code Assist) Auth");
    println!("===================================================");
    println!();
    println!("Token 文件路径: {:?}", token_path);
    println!();

    let config = AntigravityConfig {
        model: "gemini-2.5-flash".to_string(),
        token_path,
    };

    let provider = AntigravityProvider::new(config);

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        println!("正在验证认证状态...");
        println!();

        match provider.ensure_authenticated().await {
            Ok(access_token) => {
                println!("===== 认证成功! =====");
                println!();
                println!("Access Token (前60字符): {}...", &access_token[..60.min(access_token.len())]);
                println!();

                // 检查 Token 文件是否存在
                println!("获取请求头...");
                match provider.get_auth_headers().await {
                    Ok(headers) => {
                        println!("请求头构建成功!");
                        for (k, v) in &headers {
                            if k.as_str() != "authorization" {
                                println!("  {}: {:?}", k, v);
                            }
                        }
                        println!("  authorization: Bearer <token>");
                    }
                    Err(e) => {
                        println!("获取请求头失败: {:?}", e);
                    }
                }
            }
            Err(e) => {
                println!("===== 认证失败 =====");
                println!("Error: {:?}", e);
                println!();
                println!("请检查: ");
                println!("  1. 您的账户是否已订阅 Gemini Code Assist");
                println!("  2. 是否有激活的 Google Cloud Project");
                println!("  3. 网络是否可以访问 accounts.google.com");
            }
        }
    });

    println!();
    println!("按 Enter 退出...");
    let _ = std::io::stdin().read_line(&mut String::new());
}
