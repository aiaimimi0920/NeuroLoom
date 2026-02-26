use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::auth::traits::Authenticator;
use crate::provider::extension::{ModelInfo, ProviderExtension};
use crate::provider::extension::{VideoTaskState, VideoTaskStatus};

// Request & Response
#[derive(Serialize, Debug)]
struct ReplicateInput {
    prompt: String,
}

#[derive(Serialize, Debug)]
struct ReplicateVideoRequest {
    input: ReplicateInput,
}

#[derive(Deserialize, Debug)]
struct ReplicateSubmitResponse {
    id: Option<String>,
}

#[derive(Deserialize, Debug)]
struct ReplicateFetchResponse {
    #[allow(dead_code)]
    id: String,
    status: String,
    output: Option<serde_json::Value>, // It can be a string, array of strings, or null
    error: Option<String>,
}

pub struct ReplicateExtension {
    base_url: String,
}

impl ReplicateExtension {
    pub fn new() -> Self {
        Self {
            base_url: "https://api.replicate.com".to_string(),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
}

#[async_trait]
impl ProviderExtension for ReplicateExtension {
    fn id(&self) -> &str {
        "replicate_video"
    }

    async fn list_models(
        &self,
        _http: &reqwest::Client,
        _auth: &mut dyn Authenticator,
    ) -> Result<Vec<ModelInfo>> {
        Ok(vec![
            ModelInfo {
                id: "minimax/video-01".to_string(),
                description: "Minimax Video 01 via Replicate".to_string(),
            },
            ModelInfo {
                id: "luma/ray".to_string(),
                description: "Luma Ray Video via Replicate".to_string(),
            },
        ])
    }

    async fn submit_video_task(
        &self,
        http: &reqwest::Client,
        auth: &mut dyn Authenticator,
        req: &crate::primitive::PrimitiveRequest,
    ) -> Result<String> {
        // Replicate uses POST /v1/models/{model_owner}/{model_name}/predictions
        let endpoint = format!("{}/v1/models/{}/predictions", self.base_url.trim_end_matches('/'), req.model);
        
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

        let request_body = ReplicateVideoRequest {
            input: ReplicateInput { prompt },
        };

        let mut builder = http.post(&endpoint).json(&request_body);
        builder = auth.inject(builder)?;
        // Often needed for replicate
        builder = builder.header("Prefer", "wait");

        let req_obj = builder.build().map_err(|e| anyhow!("Failed to build Replicate submit request: {}", e))?;

        let response = http.execute(req_obj).await.map_err(|e| anyhow!("Network error: {}", e))?;
        let status = response.status();
        let res_text = response.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(anyhow!("Replicate API HTTP error ({}): {}", status, res_text));
        }

        let task_resp: ReplicateSubmitResponse = serde_json::from_str(&res_text)
            .map_err(|e| anyhow!("Failed to parse response: {}, body: {}", e, res_text))?;

        if let Some(task_id) = task_resp.id {
            Ok(task_id)
        } else {
            Err(anyhow!("Replicate API error: missing prediction id in response"))
        }
    }

    async fn fetch_video_task(
        &self,
        http: &reqwest::Client,
        auth: &mut dyn Authenticator,
        task_id: &str,
    ) -> Result<VideoTaskStatus> {
        let endpoint = format!("{}/v1/predictions/{}", self.base_url.trim_end_matches('/'), task_id);

        let mut builder = http.get(&endpoint);
        builder = auth.inject(builder)?;
        let req_obj = builder.build().map_err(|e| anyhow!("Failed to build Replicate fetch request: {}", e))?;

        let response = http.execute(req_obj).await.map_err(|e| anyhow!("Network error: {}", e))?;
        let status = response.status();
        let res_text = response.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(anyhow!("Replicate API HTTP fetch error ({}): {}", status, res_text));
        }

        let task_resp: ReplicateFetchResponse = serde_json::from_str(&res_text)
            .map_err(|e| anyhow!("Failed to parse response: {}", e))?;

        let state = match task_resp.status.as_str() {
            "starting" | "processing" => VideoTaskState::Processing,
            "succeeded" => VideoTaskState::Succeed,
            "failed" | "canceled" => VideoTaskState::Failed,
            _ => VideoTaskState::Processing,
        };

        let mut urls = vec![];
        if state == VideoTaskState::Succeed {
            if let Some(output) = task_resp.output {
                if let Some(url) = output.as_str() {
                    urls.push(url.to_string());
                } else if let Some(arr) = output.as_array() {
                    for v in arr {
                        if let Some(url) = v.as_str() {
                            urls.push(url.to_string());
                        }
                    }
                }
            }
        }

        Ok(VideoTaskStatus {
            id: task_id.to_string(),
            state,
            message: task_resp.error,
            video_urls: urls,
        })
    }
}
