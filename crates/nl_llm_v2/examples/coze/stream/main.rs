use nl_llm_v2::{client::LlmClient, primitive::PrimitiveRequest};
use std::io::Write;
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // The bot ID to use
    let bot_id = "7342880194848374786";

    // 1. 初始化 Coze 客户端
    let api_key = std::env::var("COZE_API_KEY").expect("请设置 COZE_API_KEY 环境变量");
    let client = LlmClient::from_preset("coze")
        .expect("Coze preset not found")
        .with_api_key(api_key)
        .build();

    // 2. 构造请求
    let mut req = PrimitiveRequest::single_user_message("给我讲一个关于勇敢小冒险家的故事。")
        .with_model(bot_id);
    req.system = Some("你是一个讲故事的专家，每次讲一个有趣的小故事。".to_string());

    println!("▶ 开始向 Coze 发送流式请求...\n");

    let mut stream = client.stream(&req).await?;

    // 3. 消费输出流
    let mut i = 0;
    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                print!("{}", chunk.content);
                std::io::stdout().flush().unwrap();
                i += 1;
            }
            Err(e) => {
                eprintln!("\n❌ 流中断: {}", e);
                break;
            }
        }
    }

    println!("\n\n✅ Coze 流输出完成，总共接收了 {} 个代码块。", i);

    Ok(())
}
