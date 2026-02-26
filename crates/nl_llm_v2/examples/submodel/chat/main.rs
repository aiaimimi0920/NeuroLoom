use tokio::io::{stdout, AsyncWriteExt};
use tokio_stream::StreamExt;
use std::env;

use nl_llm_v2::client::LlmClient;
use nl_llm_v2::primitive::PrimitiveRequest;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 强制输出无缓冲（可选）
    stdout().flush().await?;

    // 获取密钥
    let api_key = env::var("SUBMODEL_API_KEY").expect("SUBMODEL_API_KEY must be set");

    // 初始化客户端，通过预设加载 SubModel 代理模型配置
    let client = LlmClient::from_preset("submodel")
        .expect("SubModel preset not found")
        .auth(nl_llm_v2::auth::providers::api_key::ApiKeyAuth::new(&api_key))
        .build();

    println!("🚀 开始与 SubModel ('ds' -> DeepSeek) 对话测试...\n");

    // "ds" 模型将会被 SubModelModelResolver 解析为对应 DeepSeek
    let mut request = PrimitiveRequest::single_user_message("你好！请用一句话介绍你自己，加上一点赛博朋克风。")
        .with_model("qwen-thinking");
    
    request.stream = true;

    let mut stream = client.stream(&request).await?;

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(res) => {
                print!("{}", res.content);
                stdout().flush().await?;
            }
            Err(e) => {
                eprintln!("\n❌ 流式处理发生错误: {}", e);
                break;
            }
        }
    }

    println!("\n\n✅ 流式对话结束");

    Ok(())
}
