use nl_llm_v2::{LlmClient, PrimitiveRequest};
use serde_json::json;
use tokio::time::{sleep, Duration};

fn env_opt(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|s| !s.trim().is_empty())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 用法：
    //   方式 A（推荐：避免在 cargo 日志中明文打印 key）
    //     set VIDU_API_KEY=...
    //     cargo run --example vidu_video -- <prompt> <img1> [img2] [img3] ...
    //
    //   方式 B（兼容：把 key 作为第一个参数传入；不推荐，因为 cargo 会打印 Running 行）
    //     cargo run --example vidu_video -- <api_key> <prompt> <img1> [img2] [img3] ...
    //
    // 推荐：把 key 放到环境变量 VIDU_API_KEY 或 examples/vidu/.env（由 test.bat 自动加载，不提交）。

    let mut args: Vec<String> = std::env::args().skip(1).collect();

    // Vidu API key 常见前缀：vda_
    let (api_key, prompt) = match (args.get(0), env_opt("VIDU_API_KEY")) {
        // 显式传入 key（方式 B）
        (Some(first), _) if first.starts_with("vda_") => {
            let k = args.remove(0);
            let p = if !args.is_empty() {
                args.remove(0)
            } else {
                "A cinematic scene".to_string()
            };
            (k, p)
        }
        // 使用环境变量 key（方式 A）
        (_, Some(k)) => {
            let p = if !args.is_empty() {
                args.remove(0)
            } else {
                "A cinematic scene".to_string()
            };
            (k, p)
        }
        _ => {
            eprintln!("Error: VIDU_API_KEY not set.\n\nSet VIDU_API_KEY env var (recommended) or pass api_key as first arg.\n");
            std::process::exit(1);
        }
    };

    let images: Vec<String> = args
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if images.is_empty() {
        eprintln!("Error: at least 1 image url is required (for img2video/start-end/reference).\n");
        eprintln!("Example:\n  cargo run --example vidu_video -- $env:VIDU_API_KEY \"prompt\" \"https://.../a.png\" \"https://.../b.png\"");
        std::process::exit(1);
    }

    let model = env_opt("VIDU_MODEL").unwrap_or_else(|| "viduq1".to_string());

    // 构建 client（Vidu 使用 Authorization: Token <key>）
    let client = LlmClient::from_preset("vidu")
        .expect("Preset should exist")
        .auth(nl_llm_v2::auth::providers::vidu::ViduAuth::new(api_key))
        .build();

    // 构造 PrimitiveRequest：
    // - model: 可使用 viduq1/viduq2/vidu2.0/vidu1.5
    // - messages: Text + 多张 Image URL（数量决定动作：1=img2video, 2=start-end2video, >2=reference2video）
    let mut req = PrimitiveRequest::new(model);
    req.messages.push(nl_llm_v2::primitive::PrimitiveMessage {
        role: nl_llm_v2::primitive::Role::User,
        content: {
            let mut c = Vec::new();
            c.push(nl_llm_v2::primitive::PrimitiveContent::Text {
                text: prompt,
            });
            for url in images {
                c.push(nl_llm_v2::primitive::PrimitiveContent::Image {
                    url,
                    mime_type: None,
                });
            }
            c
        },
    });

    // 允许通过环境变量覆盖常用参数
    // 这些会透传到 `PrimitiveRequest.extra` 并由 ViduExtension 读取。
    //
    // 注意：ViduExtension 也支持直接读取这些环境变量（用于“业务层不改代码、只改环境”）。
    // 这里仍把它们写入 extra，是为了示例更直观。
    if let Some(duration) = env_opt("VIDU_DURATION") {
        if let Ok(n) = duration.parse::<i32>() {
            req.extra.insert("duration".into(), json!(n));
        }
    }
    if let Some(resolution) = env_opt("VIDU_RESOLUTION") {
        req.extra.insert("resolution".into(), json!(resolution));
    }
    if let Some(movement_amplitude) = env_opt("VIDU_MOVEMENT_AMPLITUDE") {
        req.extra
            .insert("movement_amplitude".into(), json!(movement_amplitude));
    }
    if let Some(seed) = env_opt("VIDU_SEED") {
        if let Ok(n) = seed.parse::<i32>() {
            req.extra.insert("seed".into(), json!(n));
        }
    }
    if let Some(bgm) = env_opt("VIDU_BGM") {
        let v = bgm.trim().to_lowercase();
        let parsed = matches!(v.as_str(), "1" | "true" | "yes" | "y" | "on");
        req.extra.insert("bgm".into(), json!(parsed));
    }
    if let Some(off_peak) = env_opt("VIDU_OFF_PEAK") {
        let v = off_peak.trim().to_lowercase();
        let parsed = matches!(v.as_str(), "1" | "true" | "yes" | "y" | "on");
        req.extra.insert("off_peak".into(), json!(parsed));
    }
    if let Some(callback_url) = env_opt("VIDU_CALLBACK_URL") {
        req.extra.insert("callback_url".into(), json!(callback_url));
    }
    if let Some(payload) = env_opt("VIDU_PAYLOAD") {
        req.extra.insert("payload".into(), json!(payload));
    }

    println!("============ Submit Vidu Video Task ============");

    let task_id = client.submit_video_task(&req).await?;
    println!(">> task_id: {}", task_id);

    loop {
        sleep(Duration::from_secs(10)).await;
        print!(">> polling... ");

        match client.fetch_video_task(&task_id).await {
            Ok(status) => {
                println!("{:?}", status.state);
                match status.state {
                    nl_llm_v2::provider::extension::VideoTaskState::Succeed => {
                        println!("\n============ Done ============");
                        for (i, url) in status.video_urls.iter().enumerate() {
                            println!("video {}: {}", i + 1, url);
                        }
                        break;
                    }
                    nl_llm_v2::provider::extension::VideoTaskState::Failed => {
                        println!("failed: {:?}", status.message);
                        break;
                    }
                    _ => {}
                }
            }
            Err(e) => {
                println!("poll error: {}", e);
            }
        }
    }

    Ok(())
}
