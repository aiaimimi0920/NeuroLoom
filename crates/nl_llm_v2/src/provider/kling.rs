use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::auth::traits::Authenticator;
use crate::provider::extension::{ProviderExtension, VideoTaskState, VideoTaskStatus, ModelInfo};
use crate::primitive::{PrimitiveRequest, PrimitiveContent};

pub struct KlingExtension;

impl KlingExtension {
    pub fn new() -> Self {
        Self
    }
}

// ----------------------------------------------------
// Request Structs
// ----------------------------------------------------
#[derive(Debug, Serialize)]
struct KlingVideoRequest {
    model: String,
    prompt: String,
    // 如果有图片，转为图生视频参数
    #[serde(skip_serializing_if = "Option::is_none")]
    image: Option<String>,
    // cfg_scale: f32, // 可选扩展
}

// ----------------------------------------------------
// Response Structs
// ----------------------------------------------------
#[derive(Debug, Deserialize)]
struct KlingTaskData {
    task_id: String,
    task_status: String,      // submitted, processing, succeed, failed
    task_status_msg: Option<String>,
    task_result: Option<KlingTaskResult>,
}

#[derive(Debug, Deserialize)]
struct KlingTaskResult {
    videos: Vec<KlingVideoObj>,
}

#[derive(Debug, Deserialize)]
struct KlingVideoObj {
    id: String,
    url: String,
    duration: String,
}

#[derive(Debug, Deserialize)]
struct KlingGenericResponse {
    code: i32,
    message: String,
    data: KlingTaskData,
}

#[async_trait]
impl ProviderExtension for KlingExtension {
    fn id(&self) -> &str {
        "kling"
    }

    async fn list_models(
        &self,
        _http: &reqwest::Client,
        _auth: &mut dyn Authenticator
    ) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(vec![
            ModelInfo {
                id: "kling-v1".to_string(),
                description: "Kling v1".to_string(),
            },
            ModelInfo {
                id: "kling-v1-6".to_string(),
                description: "Kling v1.6".to_string(),
            },
            ModelInfo {
                id: "kling-v2-master".to_string(),
                description: "Kling v2 Master".to_string(),
            },
            ModelInfo {
                id: "kling-video-o1".to_string(),
                description: "Kling Video O1".to_string(),
            },
            ModelInfo {
                id: "kling-v3-omni".to_string(),
                description: "Kling V3 Omni".to_string(),
            },
        ])
    }

    async fn submit_video_task(
        &self,
        http: &reqwest::Client,
        auth: &mut dyn Authenticator,
        req: &PrimitiveRequest,
    ) -> anyhow::Result<String> {
        let mut prompt = String::new();
        let mut image_url: Option<String> = None;

        // 提取 prompt 和 image
        for msg in &req.messages {
            for content in &msg.content {
                match content {
                    PrimitiveContent::Text { text } => {
                        if !prompt.is_empty() {
                            prompt.push('\n');
                        }
                        prompt.push_str(text);
                    }
                    PrimitiveContent::Image { url, .. } => {
                        image_url = Some(url.clone());
                    }
                    _ => {}
                }
            }
        }

        if prompt.is_empty() {
            prompt = "A cinematic view".to_string(); // fallback
        }

        let is_image2video = image_url.is_some();
        let endpoint = if is_image2video {
            "https://api-beijing.klingai.com/v1/videos/image2video"
        } else {
            "https://api-beijing.klingai.com/v1/videos/text2video"
        };

        let request_body = KlingVideoRequest {
            model: req.model.clone(),
            prompt,
            image: image_url,
        };

        // 注入 Auth (Kling 内部其实就是 JWT Bearer)
        let mut req_builder = http.post(endpoint).json(&request_body);
        req_builder = auth.inject(req_builder)?;
        
        // 伪装 UA 被要求的情况（按照 new-api 适配器中的参考值）
        req_builder = req_builder.header("User-Agent", "kling-sdk/1.0");

        let resp = req_builder.send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Kling API error ({}): {}", status, text));
        }

        let parsed: KlingGenericResponse = resp.json().await?;
        if parsed.code != 0 {
            return Err(anyhow::anyhow!("Kling returned error code {}: {}", parsed.code, parsed.message));
        }

        Ok(parsed.data.task_id)
    }

    async fn fetch_video_task(
        &self,
        http: &reqwest::Client,
        auth: &mut dyn Authenticator,
        task_id: &str,
    ) -> anyhow::Result<VideoTaskStatus> {
        // 由于获取状态时，文生和图生的查询路由不同，但 task_id 本身不包含类型
        // 如果我们不知道它是文生还是图生，尝试依次探测，或者假定文生视频通常也能拿到图生结果。
        // （Kling 区分了 /v1/videos/text2video/{task_id} 和 /v1/videos/image2video/{task_id}）
        
        let endpoint1 = format!("https://api-beijing.klingai.com/v1/videos/text2video/{}", task_id);
        let endpoint2 = format!("https://api-beijing.klingai.com/v1/videos/image2video/{}", task_id);

        let mut req_builder = http.get(&endpoint1);
        req_builder = auth.inject(req_builder)?;
        req_builder = req_builder.header("User-Agent", "kling-sdk/1.0");

        let mut resp = req_builder.send().await?;

        // 试探 fallback 逻辑
        if resp.status().as_u16() == 404 || resp.status().as_u16() == 400 {
            let mut req2 = http.get(&endpoint2);
            req2 = auth.inject(req2)?;
            req2 = req2.header("User-Agent", "kling-sdk/1.0");
            let resp2 = req2.send().await?;
            if resp2.status().is_success() {
                resp = resp2;
            }
        }

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Failed to fetch video status: {}", text));
        }

        let parsed: KlingGenericResponse = resp.json().await?;
        if parsed.code != 0 {
            return Err(anyhow::anyhow!("Kling fetch error {}: {}", parsed.code, parsed.message));
        }

        let state = match parsed.data.task_status.as_str() {
            "submitted" => VideoTaskState::Submitted,
            "processing" => VideoTaskState::Processing,
            "succeed" | "success" => VideoTaskState::Succeed,
            "failed" | "fail" => VideoTaskState::Failed,
            _ => VideoTaskState::Processing, // 默认 processing
        };

        let mut video_urls = Vec::new();
        if let Some(res) = parsed.data.task_result {
            for v in res.videos {
                if !v.url.is_empty() {
                    video_urls.push(v.url);
                }
            }
        }

        Ok(VideoTaskStatus {
            id: task_id.to_string(),
            state,
            message: parsed.data.task_status_msg,
            video_urls,
        })
    }
}
