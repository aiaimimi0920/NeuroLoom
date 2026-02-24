//! openai 平台测试 - auth
//!
//! 运行方式: cargo run --example openai_auth
//! 或直接运行: test.bat

use nl_llm_v2::{LlmClient, PrimitiveRequest};
use nl_llm_v2::protocol::error::ErrorKind;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // 强制使用错误的 API Key 以测试错误捕获机制
    let api_key = "sk-invalid-key-for-testing-auth".to_string();

    let client = LlmClient::from_preset("openai")
        .expect("Preset should exist")
        .with_api_key(api_key)
        .build();

    let req = PrimitiveRequest::single_user_message("Hello!")
        .with_model("gpt-3.5-turbo");

    println!("发送测试请求 (model: gpt-3.5-turbo)...\n");

    match client.complete(&req).await {
        Ok(_) => {
            println!("❌ 预期外结果: 错误的 Token 竟然请求成功了?");
        }
        Err(e) => {
            // 我们期望 Protocol 能够正确解析由于 Token 错误导致的 HTTP 401 并映射为 ErrorKind::Authentication
            // 我们期望 Protocol 能够正确解析由于 Token 错误导致的 HTTP 401 
            // 如果由于 anyhow::Error 的原因无法直接 downcast，我们也可检查其 Display 输出
            let err_str = e.to_string();
            if err_str.contains("[Authentication]") {
                println!("✅ 验证成功: OpenAI Protocol 正常捕获认证错误 (ErrorKind::Authentication)");
                println!("详细原因: {}", err_str);
            } else if let Some(std_err) = e.downcast_ref::<nl_llm_v2::protocol::error::StandardError>() {
                if std_err.kind == ErrorKind::Authentication {
                    println!("✅ 验证成功: OpenAI Protocol 正常捕获认证错误 (ErrorKind::Authentication)");
                    println!("详细原因: {}", std_err.message);
                } else {
                    println!("❌ 预期外错误类型: {:?}", std_err.kind);
                }
            } else {
                println!("❌ 预期外错误格式: {}", e);
            }
        }
    }

    Ok(())
}
