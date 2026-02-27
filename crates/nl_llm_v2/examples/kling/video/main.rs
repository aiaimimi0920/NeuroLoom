use nl_llm_v2::{LlmClient, PrimitiveRequest};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 从环境变量中获取可灵的 AccessKey 和 SecretKey (用 '|' 拼接)
    let token = std::env::var("KLING_CREDENTIALS")
        .expect("KLING_CREDENTIALS 环境变量未设置, 格式: AccessKey|SecretKey");

    // 1. 使用预设构建 Client，并注入 Auth
    let client = LlmClient::from_preset("kling")
        .expect("Preset should exist")
        .auth(nl_llm_v2::auth::providers::kling::KlingAuth::new(token))
        .build();

    // 2. 构造原语请求
    let mut req = PrimitiveRequest::new("kling-v1"); // 可以是 kling-v1, kling-v1-6, kling-v2-master
    req.messages.push(nl_llm_v2::primitive::PrimitiveMessage {
        role: nl_llm_v2::primitive::Role::User,
        content: vec![nl_llm_v2::primitive::PrimitiveContent::Text {
            text: "一个赛博朋克风格的城市夜景，街边有霓虹灯招牌，下着小雨".to_string(),
        }],
    });

    println!("============ 提交可灵视频生成任务 ============");
<<<<<<< Updated upstream
    println!("提示词: {}", req.messages[0].content[0].as_text().unwrap());

=======
    
>>>>>>> Stashed changes
    // 3. 提交任务
    let task_id = client.submit_video_task(&req).await?;
    println!(">> 任务提交成功！Task ID: {}", task_id);

    // 4. 轮询任务状态
    loop {
        sleep(Duration::from_secs(10)).await;
        print!(">> 正在查询状态... ");

        match client.fetch_video_task(&task_id).await {
            Ok(status) => {
                println!("{:?}", status.state);

                match status.state {
                    nl_llm_v2::provider::extension::VideoTaskState::Succeed => {
                        println!("\n============ 视频生成完成 ============");
                        for (i, url) in status.video_urls.iter().enumerate() {
                            println!("视频 {}: {}", i + 1, url);
                        }
                        break;
                    }
                    nl_llm_v2::provider::extension::VideoTaskState::Failed => {
                        println!("任务失败: {:?}", status.message);
                        break;
                    }
                    _ => {
                        // 继续等待 (Submitted 或 Processing)
                    }
                }
            }
            Err(e) => {
                println!("拉取状态失败 (可能在重试中): {}", e);
            }
        }
    }

    Ok(())
}
