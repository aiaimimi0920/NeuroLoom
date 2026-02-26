use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    data: Option<SoraSubmitData>,
}

#[derive(Deserialize, Debug)]
struct SoraSubmitData {
    id: Option<String>,
    task_id: Option<String>,
}

#[derive(Deserialize, Debug)]
struct SoraFetchResponse {
    #[allow(dead_code)]
    id: String,
    status: String,
    // When completed, video url typically goes here if the provider proxy supports it directly
    video_url: Option<String>,
    // New API format fallback
    content: Option<SoraVideoContent>,
    error: Option<SoraError>,
    message: Option<String>,
}

#[derive(Deserialize, Debug)]
struct SoraVideoContent {
    video_url: Option<String>,
    video_urls: Option<Vec<String>>,
}

#[derive(Deserialize, Debug)]
struct SoraError {
    message: Option<String>,
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
        "sora"
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
        let base_url = sora_base_url(req, &self.base_url);
        let endpoint = format!("{}/v1/videos", base_url);

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

        if prompt.trim().is_empty() {
            return Err(anyhow!("Sora request prompt is empty"));
        }

        let request_body = SoraVideoRequest {
            model: req.model.clone(),
            prompt,
        };

        let mut builder = http.post(&endpoint).json(&request_body);
        builder = auth.inject(builder)?;
        let req_obj = builder
            .build()
            .map_err(|e| anyhow!("Failed to build Sora submit request: {}", e))?;

        let response = http
            .execute(req_obj)
            .await
            .map_err(|e| anyhow!("Network error: {}", e))?;
        let status = response.status();
        let res_text = response.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(anyhow!("Sora API HTTP error ({}): {}", status, res_text));
        }

        let task_resp: SoraSubmitResponse = serde_json::from_str(&res_text)
            .map_err(|e| anyhow!("Failed to parse response: {}, bodies: {}", e, res_text))?;

        // Fallback for different proxy aliases (id or task_id)
        let task_id = task_resp
            .id
            .or(task_resp.task_id)
            .or_else(|| task_resp.data.and_then(|d| d.id.or(d.task_id)));

        if let Some(task_id) = task_id {
            Ok(compose_task_handle(&base_url, &task_id))
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
        let (base_url, raw_task_id) = parse_task_handle(task_id, &self.base_url);
        let endpoint = format!("{}/v1/videos/{}", base_url, raw_task_id);

        let mut builder = http.get(&endpoint);
        builder = auth.inject(builder)?;
        let req_obj = builder
            .build()
            .map_err(|e| anyhow!("Failed to build Sora fetch request: {}", e))?;

        let response = http
            .execute(req_obj)
            .await
            .map_err(|e| anyhow!("Network error: {}", e))?;
        let status = response.status();
        let res_text = response.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(anyhow!(
                "Sora API HTTP fetch error ({}): {}",
                status,
                res_text
            ));
        }

        let task_resp: SoraFetchResponse = serde_json::from_str(&res_text)
            .map_err(|e| anyhow!("Failed to parse response: {}", e))?;

        let state = match task_resp.status.as_str() {
            "submitted" => VideoTaskState::Submitted,
            "queued" | "pending" | "processing" | "in_progress" => VideoTaskState::Processing,
            "succeeded" | "completed" => VideoTaskState::Succeed,
            "failed" | "cancelled" => VideoTaskState::Failed,
            _ => VideoTaskState::Processing,
        };

        let mut urls = vec![];
        if state == VideoTaskState::Succeed {
            if let Some(direct_url) = task_resp.video_url {
                urls.push(direct_url);
            } else if let Some(content) = task_resp.content {
                if let Some(content_url) = content.video_url {
                    urls.push(content_url);
                }
                if let Some(more_urls) = content.video_urls {
                    urls.extend(more_urls);
                }
            }
        }

        let message = task_resp
            .error
            .and_then(|e| e.message)
            .or(task_resp.message);

        Ok(VideoTaskStatus {
            id: raw_task_id.to_string(),
            state,
            message,
            video_urls: urls,
        })
    }
}

const TASK_ID_DELIMITER: &str = "::sora_base::";

fn compose_task_handle(base_url: &str, task_id: &str) -> String {
    format!(
        "{}{}{}",
        base_url.trim_end_matches('/'),
        TASK_ID_DELIMITER,
        task_id
    )
}

fn parse_task_handle<'a>(task_id: &'a str, default_base_url: &'a str) -> (&'a str, &'a str) {
    if let Some((base, raw_task_id)) = task_id.rsplit_once(TASK_ID_DELIMITER) {
        (base, raw_task_id)
    } else {
        (default_base_url.trim_end_matches('/'), task_id)
    }
}

fn sora_base_url(req: &crate::primitive::PrimitiveRequest, default_base_url: &str) -> String {
    req.extra
        .get("sora_base_url")
        .and_then(Value::as_str)
        .unwrap_or(default_base_url)
        .trim_end_matches('/')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{compose_task_handle, parse_task_handle, TASK_ID_DELIMITER};

    #[test]
    fn task_handle_roundtrip() {
        let handle = compose_task_handle("https://example.com/", "abc123");
        let (base, task_id) = parse_task_handle(&handle, "https://fallback.com");
        assert_eq!(base, "https://example.com");
        assert_eq!(task_id, "abc123");
    }

    #[test]
    fn fallback_when_legacy_task_id_without_base_url() {
        let raw = "legacy_task";
        let (base, task_id) = parse_task_handle(raw, "https://fallback.com/");
        assert_eq!(base, "https://fallback.com");
        assert_eq!(task_id, raw);
    }

    #[test]
    fn delimiter_is_stable() {
        assert_eq!(TASK_ID_DELIMITER, "::sora_base::");
    }
}
