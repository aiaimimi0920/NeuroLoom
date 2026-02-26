use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::auth::traits::Authenticator;
use crate::provider::extension::{ModelInfo, ProviderExtension};
use crate::provider::extension::{VideoTaskState, VideoTaskStatus};

// Request & Response for Sora (compatible with standard OpenAI video endpoints)
#[derive(Serialize, Debug)]
struct SoraVideoRequest {
    model: String,
    prompt: String,
}

#[derive(Deserialize, Debug)]
struct SoraSubmitResponse {
    id: Option<String>,
    task_id: Option<String>, // Some channels might use this alias
}

#[derive(Deserialize, Debug)]
struct SoraFetchResponse {
    #[allow(dead_code)]
    id: String,
    status: String,
    // When completed, video url typically goes here if the provider proxy supports it directly
    video_url: Option<String>, 
    // New API format fallback
    content: Option<SoraVideoContent>
}

#[derive(Deserialize, Debug)]
struct SoraVideoContent {
    video_url: String,
}

pub struct SoraExtension {
    base_url: String,
}

impl SoraExtension {
    pub fn new() -> Self {
        Self {
            base_url: "https://api.openai.com".to_string(),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
}

#[async_trait]
impl ProviderExtension for SoraExtension {
    fn id(&self) -> &str {
        "sora_video"
    }

    async fn list_models(
        &self,
        _http: &reqwest::Client,
        _auth: &mut dyn Authenticator,
    ) -> Result<Vec<ModelInfo>> {
        Ok(vec![
            ModelInfo {
                id: "sora-2".to_string(),
                description: "Sora Version 2".to_string(),
            },
            ModelInfo {
                id: "sora-2-pro".to_string(),
                description: "Sora Version 2 Pro".to_string(),
            },
        ])
    }

    async fn submit_video_task(
        &self,
        http: &reqwest::Client,
        auth: &mut dyn Authenticator,
        req: &crate::primitive::PrimitiveRequest,
    ) -> Result<String> {
        let endpoint = format!("{}/v1/videos", self.base_url.trim_end_matches('/'));
        
        // Extract unified text prompt 
        let mut prompt = String::new();
        for msg in &req.messages {
            for content in &msg.content {
                if let crate::primitive::PrimitiveContent::Text { text } = content {
                    if !prompt.is_empty() {
                        prompt.push('\n');
                    }
                    prompt.push_str(text);
                }
            }
        }

        let request_body = SoraVideoRequest {
            model: req.model.clone(),
            prompt,
        };

        let mut builder = http.post(&endpoint).json(&request_body);
        builder = auth.inject(builder)?;
        let req_obj = builder.build().map_err(|e| anyhow!("Failed to build Sora submit request: {}", e))?;

        let response = http.execute(req_obj).await.map_err(|e| anyhow!("Network error: {}", e))?;
        let status = response.status();
        let res_text = response.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(anyhow!("Sora API HTTP error ({}): {}", status, res_text));
        }

        let task_resp: SoraSubmitResponse = serde_json::from_str(&res_text)
            .map_err(|e| anyhow!("Failed to parse response: {}, bodies: {}", e, res_text))?;

        // Fallback for different proxy aliases (id or task_id)
        if let Some(task_id) = task_resp.id.or(task_resp.task_id) {
            Ok(task_id)
        } else {
            Err(anyhow!("Sora API error: missing task id in response"))
        }
    }

    async fn fetch_video_task(
        &self,
        http: &reqwest::Client,
        auth: &mut dyn Authenticator,
        task_id: &str,
    ) -> Result<VideoTaskStatus> {
        let endpoint = format!("{}/v1/videos/{}", self.base_url.trim_end_matches('/'), task_id);

        let mut builder = http.get(&endpoint);
        builder = auth.inject(builder)?;
        let req_obj = builder.build().map_err(|e| anyhow!("Failed to build Sora fetch request: {}", e))?;

        let response = http.execute(req_obj).await.map_err(|e| anyhow!("Network error: {}", e))?;
        let status = response.status();
        let res_text = response.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(anyhow!("Sora API HTTP fetch error ({}): {}", status, res_text));
        }

        let task_resp: SoraFetchResponse = serde_json::from_str(&res_text)
            .map_err(|e| anyhow!("Failed to parse response: {}", e))?;

        let state = match task_resp.status.as_str() {
            "queued" | "pending" | "processing" | "in_progress" => VideoTaskState::Processing,
            "succeeded" | "completed" => VideoTaskState::Succeed,
            "failed" | "cancelled" => VideoTaskState::Failed,
            _ => VideoTaskState::Failed,
        };

        let mut urls = vec![];
        if state == VideoTaskState::Succeed {
            if let Some(direct_url) = task_resp.video_url {
                urls.push(direct_url);
            } else if let Some(content_url) = task_resp.content.map(|c| c.video_url) {
                 urls.push(content_url);
            }
        }

        Ok(VideoTaskStatus {
            id: task_id.to_string(),
            state,
            message: None,
            video_urls: urls,
        })
    }
}
