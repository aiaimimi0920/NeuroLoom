use std::env;
use std::time::Duration;

use tokio::time::sleep;

use nl_llm_v2::client::LlmClient;
use nl_llm_v2::primitive::{PrimitiveContent, PrimitiveMessage, PrimitiveRequest, Role};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = env::var("VIDU_API_KEY").expect("VIDU_API_KEY must be set");
    let base_url =
        env::var("VIDU_API_BASE_URL").unwrap_or_else(|_| "https://api.vidu.cn".to_string());

    // 可选参数（环境变量）
    let model = env::var("VIDU_MODEL").unwrap_or_else(|_| "viduq1".to_string());
    let duration = env::var("VIDU_DURATION")
        .ok()
        .and_then(|v| v.parse::<i64>().ok());
    let resolution = env::var("VIDU_RESOLUTION").ok();
    let movement_amplitude = env::var("VIDU_MOVEMENT_AMPLITUDE").ok();
    let bgm = env::var("VIDU_BGM")
        .ok()
        .and_then(|v| v.parse::<bool>().ok());
    let seed = env::var("VIDU_SEED")
        .ok()
        .and_then(|v| v.parse::<i64>().ok());
    let callback_url = env::var("VIDU_CALLBACK_URL").ok();
    let payload = env::var("VIDU_PAYLOAD").ok();

    // 1) 构建 Client：preset + 自定义 auth + 覆盖 base_url
    let client = LlmClient::from_preset("vidu")
        .expect("vidu preset not found")
        .auth(nl_llm_v2::auth::providers::vidu::ViduApiKeyAuth::new(
            api_key,
        ))
        .with_base_url(base_url)
        .build();

    println!("🚀 开始提交 Vidu 图片生成视频任务...");

    // 2) 构造请求（必须包含至少一张图片 URL）
    let mut request = PrimitiveRequest::new(model);

    request.messages.push(PrimitiveMessage {
        role: Role::User,
        content: vec![
            PrimitiveContent::Text {
                text: "让画面产生自然的镜头运动，真实光照，电影质感".to_string(),
            },
            // TODO: 你可以把下面 URL 换成自己可访问的图片
            PrimitiveContent::Image {
                url: "https://example.com/your-image.jpg".to_string(),
                mime_type: None,
            },
        ],
    });

    // 3) 写入 extra 参数（供 ViduExtension 使用）
    if let Some(v) = duration {
        request
            .extra
            .insert("vidu_duration".into(), serde_json::json!(v));
    }
    if let Some(v) = resolution {
        request
            .extra
            .insert("vidu_resolution".into(), serde_json::json!(v));
    }
    if let Some(v) = movement_amplitude {
        request
            .extra
            .insert("vidu_movement_amplitude".into(), serde_json::json!(v));
    }
    if let Some(v) = bgm {
        request
            .extra
            .insert("vidu_bgm".into(), serde_json::json!(v));
    }
    if let Some(v) = seed {
        request
            .extra
            .insert("vidu_seed".into(), serde_json::json!(v));
    }
    if let Some(v) = callback_url {
        request
            .extra
            .insert("vidu_callback_url".into(), serde_json::json!(v));
    }
    if let Some(v) = payload {
        request
            .extra
            .insert("vidu_payload".into(), serde_json::json!(v));
    }

    // 4) 提交任务
    let task_id = client.submit_video_task(&request).await?;
    println!("✅ 任务提交成功! Task ID: {}", task_id);
    println!("⏳ 开始轮询任务状态...");

    // 5) 轮询
    loop {
        sleep(Duration::from_secs(5)).await;

        match client.fetch_video_task(&task_id).await {
            Ok(status) => match status.state {
                nl_llm_v2::provider::extension::VideoTaskState::Submitted => {
                    println!("...任务排队中 (Task ID: {})", task_id);
                }
                nl_llm_v2::provider::extension::VideoTaskState::Processing => {
                    println!("...任务处理中 (Task ID: {})", task_id);
                }
                nl_llm_v2::provider::extension::VideoTaskState::Succeed => {
                    println!("🎉 生成成功!");
                    for (i, url) in status.video_urls.iter().enumerate() {
                        println!("🔗 视频地址 [{}]: {}", i + 1, url);
                    }
                    break;
                }
                nl_llm_v2::provider::extension::VideoTaskState::Failed => {
                    println!("❌ 生成失败!");
                    if let Some(msg) = status.message {
                        println!("错误信息: {}", msg);
                    }
                    break;
                }
            },
            Err(e) => {
                eprintln!("⚠️ 查询状态出错（将重试）: {}", e);
            }
        }
    }

    Ok(())
}
