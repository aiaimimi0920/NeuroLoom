use tokio::io::{stdout, AsyncWriteExt};
use tokio::time::{sleep, Duration};
use std::env;

use nl_llm_v2::client::LlmClient;
use nl_llm_v2::primitive::PrimitiveRequest;
use nl_llm_v2::provider::extension::VideoTaskState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    stdout().flush().await?;

    let api_key = env::var("DOUBAO_API_KEY").unwrap_or_else(|_| "your_api_key_here".to_string());

    let client = LlmClient::from_preset("doubao")
        .expect("Doubao preset not found")
        .auth(nl_llm_v2::auth::providers::api_key::ApiKeyAuth::new(&api_key))
        .build();

    println!("🚀 [Doubao Video] 开始测试豆包视频大模型...");

    // Create the video request using the aliased 'doubao-video' routing
    let request = PrimitiveRequest::single_user_message("戴着帽子的老爷爷面带微笑往前走")
        .with_model("doubao-video");

    println!("📥 正在提交任务...");
    let task_id = client.submit_video_task(&request).await?;
    println!("✅ 任务提交成功！Task ID: {}", task_id);

    // Polling structure
    loop {
        print!("⏳ 正在查询进度... ");
        stdout().flush().await?;

        match client.fetch_video_task(&task_id).await {
            Ok(status) => {
                match status.state {
                    VideoTaskState::Submitted | VideoTaskState::Processing => {
                        println!("处理中...");
                    }
                    VideoTaskState::Succeed => {
                        println!("\n🎉 视频生成成功！");
                        if let Some(video_urls) = status.video_urls.first() {
                            println!("🔗 视频下载链接: {}", video_urls);
                        }
                        break;
                    }
                    VideoTaskState::Failed => {
                        println!("\n❌ 任务失败！");
                        if let Some(msg) = status.message {
                            println!("错误信息: {}", msg);
                        }
                        break;
                    }
                }
            }
            Err(e) => {
                println!("\n⚠️ 查询异常: {}", e);
            }
        }
        
        sleep(Duration::from_secs(5)).await;
    }

    Ok(())
}
