use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::auth::traits::Authenticator;
use crate::provider::extension::{ModelInfo, ProviderExtension};
use crate::provider::extension::{VideoTaskState, VideoTaskStatus};

const DOUBAO_API_URL: &str = "https://ark.cn-beijing.volces.com";

// Request & Response
#[derive(Serialize, Debug)]
struct DoubaoVideoContent {
    #[serde(rename = "type")]
    content_type: String, // "text"
    text: String,
}

#[derive(Serialize, Debug)]
struct DoubaoVideoRequest {
    model: String,
    content: Vec<DoubaoVideoContent>,
}

#[derive(Deserialize, Debug)]
struct DoubaoSubmitResponse {
    id: Option<String>,
}

#[derive(Deserialize, Debug)]
struct DoubaoFetchResponse {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    model: String,
    status: String,
    content: Option<DoubaoVideoOutput>,
    error: Option<DoubaoErrorInfo>,
}

#[derive(Deserialize, Debug)]
struct DoubaoErrorInfo {
    message: Option<String>,
}

#[derive(Deserialize, Debug)]
struct DoubaoVideoOutput {
    video_url: String,
}

pub struct DoubaoExtension;

impl DoubaoExtension {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ProviderExtension for DoubaoExtension {
    fn id(&self) -> &str {
        "doubao_video"
    }

    async fn list_models(
        &self,
        _http: &reqwest::Client,
        _auth: &mut dyn Authenticator,
    ) -> Result<Vec<ModelInfo>> {
        Ok(vec![
            ModelInfo {
                id: "doubao-seedance-1-0-pro-250528".to_string(),
                description: "Doubao Seedance Pro T2V".to_string(),
            },
            ModelInfo {
                id: "doubao-seedance-1-0-lite-t2v".to_string(),
                description: "Doubao Seedance Lite T2V".to_string(),
            },
            ModelInfo {
                id: "doubao-seedance-1-5-pro-251215".to_string(),
                description: "Doubao Seedance v1.5 Pro".to_string(),
            },
        ])
    }

    async fn submit_video_task(
        &self,
        http: &reqwest::Client,
        auth: &mut dyn Authenticator,
        req: &crate::primitive::PrimitiveRequest,
    ) -> Result<String> {
        let endpoint = format!("{}/api/v3/contents/generations/tasks", DOUBAO_API_URL);

        // Extract prompt
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

        let request_body = DoubaoVideoRequest {
            model: req.model.clone(),
            content: vec![DoubaoVideoContent {
                content_type: "text".to_string(),
                text: prompt,
            }],
        };

        let mut builder = http.post(&endpoint).json(&request_body);
        builder = auth.inject(builder)?;
        let req_obj = builder
            .build()
            .map_err(|e| anyhow!("Failed to build Doubao submit request: {}", e))?;

        let response = http
            .execute(req_obj)
            .await
            .map_err(|e| anyhow!("Network error: {}", e))?;
        let status = response.status();
        let res_text = response.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(anyhow!("Doubao API HTTP error ({}): {}", status, res_text));
        }

        let task_resp: DoubaoSubmitResponse = serde_json::from_str(&res_text)
            .map_err(|e| anyhow!("Failed to parse response: {}, bodies: {}", e, res_text))?;

        if let Some(task_id) = task_resp.id {
            Ok(task_id)
        } else {
            Err(anyhow!("Doubao API error: missing task_id in response"))
        }
    }

    async fn fetch_video_task(
        &self,
        http: &reqwest::Client,
        auth: &mut dyn Authenticator,
        task_id: &str,
    ) -> Result<VideoTaskStatus> {
        let endpoint = format!(
            "{}/api/v3/contents/generations/tasks/{}",
            DOUBAO_API_URL, task_id
        );

        let mut builder = http.get(&endpoint);
        builder = auth.inject(builder)?;
        let req_obj = builder
            .build()
            .map_err(|e| anyhow!("Failed to build Doubao fetch request: {}", e))?;

        let response = http
            .execute(req_obj)
            .await
            .map_err(|e| anyhow!("Network error: {}", e))?;
        let status = response.status();
        let res_text = response.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(anyhow!(
                "Doubao API HTTP fetch error ({}): {}",
                status,
                res_text
            ));
        }

        let task_resp: DoubaoFetchResponse = serde_json::from_str(&res_text)
            .map_err(|e| anyhow!("Failed to parse response: {}", e))?;

        let state = match task_resp.status.as_str() {
            "pending" | "queued" | "processing" | "running" => VideoTaskState::Processing,
            "succeeded" => VideoTaskState::Succeed,
            "failed" => VideoTaskState::Failed,
            _ => VideoTaskState::Failed,
        };

        let mut urls = vec![];
        if state == VideoTaskState::Succeed {
            if let Some(content) = task_resp.content {
                urls.push(content.video_url);
            }
        }

        let message = if state == VideoTaskState::Failed {
            task_resp.error.and_then(|e| e.message)
        } else {
            None
        };

        Ok(VideoTaskStatus {
            id: task_id.to_string(),
            state,
            message,
            video_urls: urls,
        })
    }
}
