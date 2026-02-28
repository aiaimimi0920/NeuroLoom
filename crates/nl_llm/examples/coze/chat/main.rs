use nl_llm::{client::LlmClient, primitive::PrimitiveRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 设置日志
    std::env::set_var("RUST_LOG", "debug");

    // The bot ID to use
    let bot_id = "7342880194848374786";

    // 1. 初始化 Coze 客户端并注入 API Key
    let api_key = std::env::var("COZE_API_KEY").expect("请设置 COZE_API_KEY 环境变量");
    let client = LlmClient::from_preset("coze")
        .expect("Coze preset not found")
        .with_api_key(api_key)
        .build();

    // 2. 构造请求
    println!("\n▶ 发送单轮请求给 Coze Bot ID...");
    let mut req = PrimitiveRequest::single_user_message("我今天感觉很累，好像做错了好多事情。")
        .with_model(bot_id);
    req.system = Some("你是一个专业的心理咨询师，每次用一句简短的话安慰用户。".to_string());

    let resp = client.complete(&req).await?;
    println!("\n✅ Coze (Non-Stream) Bot 回复: {}", resp.content);

    Ok(())
}
