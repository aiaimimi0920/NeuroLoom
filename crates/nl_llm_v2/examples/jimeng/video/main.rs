use std::env;
use std::time::Duration;
use tokio::time::sleep;

use nl_llm_v2::client::LlmClient;
use nl_llm_v2::primitive::{PrimitiveContent, PrimitiveMessage, PrimitiveRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 从环境变量获取 Access Key 和 Secret Key
    // 为了方便测试，我们会通过 test.bat 直接注入这些变量
    let ak = env::var("JIMENG_ACCESS_KEY").expect("JIMENG_ACCESS_KEY must be set");
    let sk = env::var("JIMENG_SECRET_KEY").expect("JIMENG_SECRET_KEY must be set");
    let auth_token = format!("{}|{}", ak, sk);

    // 初始化客户端
    let client = LlmClient::from_preset("jimeng")
        .expect("jimeng preset not found")
        .auth(nl_llm_v2::auth::providers::api_key::ApiKeyAuth::new(&auth_token))
        .build();

    println!("🚀 开始提交即梦视频生成任务...");

    // 构造请求，文生视频默认使用 jimeng_t2v_v30 (会由 ModelResolver 映射)
    let mut request = PrimitiveRequest::new("jimeng-v3".to_string());
    request.messages.push(PrimitiveMessage {
        role: nl_llm_v2::primitive::Role::User,
        content: vec![PrimitiveContent::Text {
            text: "一个赛博朋克风格的未来城市，霓虹灯闪烁，飞行汽车穿梭其中，电影级光影，8k画质，极其细腻".to_string(),
        }],
    });

    // 提交任务得到 Task ID
    let task_id = match client.submit_video_task(&request).await {
        Ok(id) => id,
        Err(e) => {
            eprintln!("❌ 任务提交失败: {}", e);
            return Err(e.into());
        }
    };

    println!("✅ 任务提交成功! Task ID: {}", task_id);
    println!("⏳ 开始轮询任务状态...");

    // 轮询获取任务结果
    loop {
        sleep(Duration::from_secs(5)).await;
        
        match client.fetch_video_task(&task_id).await {
            Ok(status) => {
                match status.state {
                    nl_llm_v2::provider::extension::VideoTaskState::Submitted => {
                        println!("...任务已提交并排队中 (Task ID: {})", task_id);
                    }
                    nl_llm_v2::provider::extension::VideoTaskState::Processing => {
                        println!("...任务处理中 (Task ID: {})", task_id);
                    }
                    nl_llm_v2::provider::extension::VideoTaskState::Succeed => {
                        println!("🎉 任务生成成功!");
                        if let Some(msg) = status.message {
                            println!("消息: {}", msg);
                        }
                        for (i, url) in status.video_urls.iter().enumerate() {
                            println!("🔗 视频地址 [{}]: {}", i + 1, url);
                        }
                        break;
                    }
                    nl_llm_v2::provider::extension::VideoTaskState::Failed => {
                        println!("❌ 任务生成失败!");
                        if let Some(msg) = status.message {
                            println!("错误信息: {}", msg);
                        }
                        break;
                    }
                }
            }
            Err(e) => {
                eprintln!("⚠️ 查询状态出错: {}", e);
                // 暂时忽略网络错误，继续重试
            }
        }
    }

    Ok(())
}
