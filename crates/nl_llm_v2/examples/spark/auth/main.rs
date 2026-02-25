//! 讯飞星火 鉴权诊断
//!
//! 运行方式: cargo run -p nl_llm_v2 --example spark_auth -- <api_password|api_key:api_secret>

use nl_llm_v2::{Capability, LlmClient, PrimitiveRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("SPARK_API_KEY")
        .or_else(|_| std::env::args().nth(1).ok_or(()))
        .unwrap_or_else(|_| {
            eprintln!("需要提供 SPARK_API_KEY (推荐 APIPassword，兼容 APIKey:APISecret)");
            std::process::exit(1);
        });

    println!("========================================");
    println!("  讯飞星火 鉴权诊断");
    println!("========================================\n");

    let client = LlmClient::from_preset("spark_x")
        .expect("Preset should exist")
        .with_spark_auth(&api_key)
        .build();

    // 测试模型解析
    let model = client.resolve_model("ultra");
    println!("模型 'ultra' 解析为: {}", model);
    println!("上下文长度: {} tokens", client.max_context("ultra"));
    println!(
        "Chat 能力: {}",
        if client.has_capability("ultra", Capability::CHAT) {
            "✅"
        } else {
            "❌"
        }
    );
    println!(
        "Tools 能力: {}",
        if client.has_capability("ultra", Capability::TOOLS) {
            "✅"
        } else {
            "❌"
        }
    );

    // 发送测试请求
    println!("\n--- 鉴权测试 ---");
    let req = PrimitiveRequest::single_user_message("你好，请回复'鉴权成功'四个字。");
    match client.complete(&req).await {
        Ok(resp) => {
            println!("✅ 鉴权成功！");
            println!("AI 回复: {}", resp.content);
            if let Some(usage) = resp.usage {
                println!(
                    "[Token 用量: prompt={}, completion={}, total={}]",
                    usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
                );
            }
        }
        Err(e) => {
            eprintln!("❌ 鉴权失败: {}", e);
            eprintln!("\n排查建议:");
            eprintln!("  1. 优先使用 APIPassword；如用旧格式，确认 APIKey:APISecret 中间有冒号");
            eprintln!("  2. 前往 https://console.xfyun.cn/services/cbm 检查服务状态");
            eprintln!("  3. 确认 Spark 服务已开通且未欠费");
            std::process::exit(1);
        }
    }

    Ok(())
}
