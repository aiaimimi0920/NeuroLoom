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

#[derive(Deserialize, Debug)]
struct ReplicateModelListResponse {
    results: Vec<ReplicateModelItem>,
}

#[derive(Deserialize, Debug)]
struct ReplicateModelItem {
    owner: Option<String>,
    name: Option<String>,
    description: Option<String>,
}

fn static_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "minimax/video-01".to_string(),
            description: "Minimax Video 01 via Replicate".to_string(),
        },
        ModelInfo {
            id: "luma/ray".to_string(),
            description: "Luma Ray Video via Replicate".to_string(),
        },
    ]
}

fn collect_prompt(req: &crate::primitive::PrimitiveRequest) -> String {
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
    prompt
}

fn extract_video_urls(output: Option<serde_json::Value>) -> Vec<String> {
    let Some(output) = output else {
        return vec![];
    };

    if let Some(url) = output.as_str() {
        return vec![url.to_string()];
    }

    if let Some(arr) = output.as_array() {
        return arr
            .iter()
            .filter_map(|v| {
                if let Some(url) = v.as_str() {
                    return Some(url.to_string());
                }

                v.get("url")
                    .and_then(|url| url.as_str())
                    .map(|url| url.to_string())
            })
            .collect();
    }

    output
        .get("url")
        .and_then(|url| url.as_str())
        .map(|url| vec![url.to_string()])
        .unwrap_or_default()
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
        "replicate"
    }

    async fn list_models(
        &self,
        http: &reqwest::Client,
        auth: &mut dyn Authenticator,
    ) -> Result<Vec<ModelInfo>> {
        let endpoint = format!("{}/v1/models", self.base_url.trim_end_matches('/'));

        let mut builder = http.get(endpoint);
        builder = auth.inject(builder)?;

        let response = http.execute(builder.build()?).await;
        let Ok(response) = response else {
            return Ok(static_models());
        };

        if !response.status().is_success() {
            return Ok(static_models());
        }

        let parsed: ReplicateModelListResponse = response.json().await?;
        let models: Vec<ModelInfo> = parsed
            .results
            .into_iter()
            .filter_map(|item| {
                let owner = item.owner?;
                let name = item.name?;
                Some(ModelInfo {
                    id: format!("{owner}/{name}"),
                    description: item
                        .description
                        .unwrap_or_else(|| format!("{name} via Replicate")),
                })
            })
            .collect();

        if models.is_empty() {
            Ok(static_models())
        } else {
            Ok(models)
        }
    }

    async fn submit_video_task(
        &self,
        http: &reqwest::Client,
        auth: &mut dyn Authenticator,
        req: &crate::primitive::PrimitiveRequest,
    ) -> Result<String> {
        // Replicate uses POST /v1/models/{model_owner}/{model_name}/predictions
        let endpoint = format!(
            "{}/v1/models/{}/predictions",
            self.base_url.trim_end_matches('/'),
            req.model
        );

        let prompt = collect_prompt(req);

        let request_body = ReplicateVideoRequest {
            input: ReplicateInput { prompt },
        };

        let mut builder = http.post(&endpoint).json(&request_body);
        builder = auth.inject(builder)?;
        // Often needed for replicate
        builder = builder.header("Prefer", "wait");

        let req_obj = builder
            .build()
            .map_err(|e| anyhow!("Failed to build Replicate submit request: {}", e))?;

        let response = http
            .execute(req_obj)
            .await
            .map_err(|e| anyhow!("Network error: {}", e))?;
        let status = response.status();
        let res_text = response.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(anyhow!(
                "Replicate API HTTP error ({}): {}",
                status,
                res_text
            ));
        }

        let task_resp: ReplicateSubmitResponse = serde_json::from_str(&res_text)
            .map_err(|e| anyhow!("Failed to parse response: {}, body: {}", e, res_text))?;

        if let Some(task_id) = task_resp.id {
            Ok(task_id)
        } else {
            Err(anyhow!(
                "Replicate API error: missing prediction id in response"
            ))
        }
    }

    async fn fetch_video_task(
        &self,
        http: &reqwest::Client,
        auth: &mut dyn Authenticator,
        task_id: &str,
    ) -> Result<VideoTaskStatus> {
        let endpoint = format!(
            "{}/v1/predictions/{}",
            self.base_url.trim_end_matches('/'),
            task_id
        );

        let mut builder = http.get(&endpoint);
        builder = auth.inject(builder)?;
        let req_obj = builder
            .build()
            .map_err(|e| anyhow!("Failed to build Replicate fetch request: {}", e))?;

        let response = http
            .execute(req_obj)
            .await
            .map_err(|e| anyhow!("Network error: {}", e))?;
        let status = response.status();
        let res_text = response.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(anyhow!(
                "Replicate API HTTP fetch error ({}): {}",
                status,
                res_text
            ));
        }

        let task_resp: ReplicateFetchResponse = serde_json::from_str(&res_text)
            .map_err(|e| anyhow!("Failed to parse response: {}", e))?;

        let state = match task_resp.status.as_str() {
            "starting" | "processing" => VideoTaskState::Processing,
            "succeeded" => VideoTaskState::Succeed,
            "failed" | "canceled" => VideoTaskState::Failed,
            _ => VideoTaskState::Processing,
        };

        let urls = if state == VideoTaskState::Succeed {
            extract_video_urls(task_resp.output)
        } else {
            vec![]
        };

        Ok(VideoTaskStatus {
            id: task_id.to_string(),
            state,
            message: task_resp.error,
            video_urls: urls,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::extract_video_urls;
    use serde_json::json;

    #[test]
    fn extract_urls_from_string_or_array_or_object() {
        assert_eq!(
            extract_video_urls(Some(json!("https://example.com/a.mp4"))),
            vec!["https://example.com/a.mp4".to_string()]
        );

        assert_eq!(
            extract_video_urls(Some(json!([
                "https://example.com/a.mp4",
                {"url": "https://example.com/b.mp4"},
                123
            ]))),
            vec![
                "https://example.com/a.mp4".to_string(),
                "https://example.com/b.mp4".to_string()
            ]
        );

        assert_eq!(
            extract_video_urls(Some(json!({"url": "https://example.com/c.mp4"}))),
            vec!["https://example.com/c.mp4".to_string()]
        );
    }
}
