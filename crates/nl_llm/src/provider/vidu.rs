use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::auth::traits::Authenticator;
use crate::primitive::{PrimitiveContent, PrimitiveRequest};
use crate::provider::extension::{ModelInfo, ProviderExtension};
use crate::provider::extension::{VideoTaskState, VideoTaskStatus};

/// Vidu 异步视频任务扩展（v0）
///
/// - Submit: `POST {base}/ent/v2/img2video`
/// - Fetch: `GET  {base}/ent/v2/tasks/{task_id}/creations`
/// - Auth:  `Authorization: Token <api_key>`
///
/// 字段与状态映射参考：`references/new-api/relay/channel/task/vidu/adaptor.go`
pub struct ViduExtension {
    base_url: String,
}

impl ViduExtension {
    pub fn new() -> Self {
        Self {
            base_url: "https://api.vidu.cn".to_string(),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    fn submit_endpoint(&self) -> String {
        format!("{}/ent/v2/img2video", self.base_url.trim_end_matches('/'))
    }

    fn fetch_endpoint(&self, task_id: &str) -> String {
        format!(
            "{}/ent/v2/tasks/{}/creations",
            self.base_url.trim_end_matches('/'),
            task_id
        )
    }
}

#[derive(Debug, Serialize)]
struct ViduSubmitRequest {
    model: String,
    images: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    prompt: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    duration: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    seed: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    resolution: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    movement_amplitude: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    bgm: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    payload: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    callback_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ViduSubmitResponse {
    task_id: String,
    state: String,
}

#[derive(Debug, Deserialize)]
struct ViduTaskCreation {
    #[serde(default)]
    url: Option<String>,

    #[serde(default)]
    cover_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ViduFetchResponse {
    state: String,

    #[serde(default)]
    err_code: Option<String>,

    #[serde(default)]
    payload: Option<String>,

    #[serde(default)]
    creations: Vec<ViduTaskCreation>,
}

fn map_state(state: &str) -> Result<VideoTaskState> {
    Ok(match state {
        "created" | "queueing" => VideoTaskState::Submitted,
        "processing" => VideoTaskState::Processing,
        "success" => VideoTaskState::Succeed,
        "failed" => VideoTaskState::Failed,
        other => return Err(anyhow!("unknown vidu task state: {}", other)),
    })
}

#[async_trait]
impl ProviderExtension for ViduExtension {
    fn id(&self) -> &str {
        "vidu"
    }

    async fn list_models(
        &self,
        _http: &reqwest::Client,
        _auth: &mut dyn Authenticator,
    ) -> Result<Vec<ModelInfo>> {
        // v0：静态兜底。后续如官方提供 models API 可再接入。
        Ok(vec![
            ModelInfo {
                id: "viduq2".to_string(),
                description: "Vidu Q2".to_string(),
            },
            ModelInfo {
                id: "viduq1".to_string(),
                description: "Vidu Q1".to_string(),
            },
            ModelInfo {
                id: "vidu2.0".to_string(),
                description: "Vidu 2.0".to_string(),
            },
            ModelInfo {
                id: "vidu1.5".to_string(),
                description: "Vidu 1.5".to_string(),
            },
        ])
    }

    async fn submit_video_task(
        &self,
        http: &reqwest::Client,
        auth: &mut dyn Authenticator,
        req: &PrimitiveRequest,
    ) -> Result<String> {
        // 解析 prompt 与 images
        let mut prompt_parts: Vec<String> = Vec::new();
        let mut images: Vec<String> = Vec::new();

        for msg in &req.messages {
            for c in &msg.content {
                match c {
                    PrimitiveContent::Text { text } => {
                        if !text.trim().is_empty() {
                            prompt_parts.push(text.clone());
                        }
                    }
                    PrimitiveContent::Image { url, .. } => {
                        if !url.trim().is_empty() {
                            images.push(url.clone());
                        }
                    }
                    _ => {}
                }
            }
        }

        if images.is_empty() {
            return Err(anyhow!("Vidu img2video requires at least 1 image URL"));
        }

        // 从 extra 中读取可选参数（字段名沿用 VIDU_* 环境变量语义）
        // 注意：这些都不是协议层字段，不要污染 PrimitiveParameters。
        let duration = req
            .extra
            .get("vidu_duration")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32);

        let seed = req
            .extra
            .get("vidu_seed")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32);

        let resolution = req
            .extra
            .get("vidu_resolution")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let movement_amplitude = req
            .extra
            .get("vidu_movement_amplitude")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let bgm = req.extra.get("vidu_bgm").and_then(|v| v.as_bool());

        let payload = req
            .extra
            .get("vidu_payload")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let callback_url = req
            .extra
            .get("vidu_callback_url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let submit_req = ViduSubmitRequest {
            model: req.model.clone(),
            images,
            prompt: if prompt_parts.is_empty() {
                None
            } else {
                Some(prompt_parts.join("\n"))
            },
            duration,
            seed,
            resolution,
            movement_amplitude,
            bgm,
            payload,
            callback_url,
        };

        let mut builder = http.post(self.submit_endpoint()).json(&submit_req);
        builder = auth.inject(builder)?;

        let resp = builder.send().await?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(anyhow!("Vidu submit HTTP error ({}): {}", status, text));
        }

        let parsed: ViduSubmitResponse = serde_json::from_str(&text).map_err(|e| {
            anyhow!(
                "failed to parse Vidu submit response: {}, body: {}",
                e,
                text
            )
        })?;

        // 提交就失败，直接报错
        if parsed.state == "failed" {
            return Err(anyhow!("vidu submit failed, task_id={}", parsed.task_id));
        }

        Ok(parsed.task_id)
    }

    async fn fetch_video_task(
        &self,
        http: &reqwest::Client,
        auth: &mut dyn Authenticator,
        task_id: &str,
    ) -> Result<VideoTaskStatus> {
        let mut builder = http.get(self.fetch_endpoint(task_id));
        builder = auth.inject(builder)?;

        let resp = builder.send().await?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(anyhow!("Vidu fetch HTTP error ({}): {}", status, text));
        }

        let parsed: ViduFetchResponse = serde_json::from_str(&text)
            .map_err(|e| anyhow!("failed to parse Vidu fetch response: {}, body: {}", e, text))?;

        let state = map_state(&parsed.state)?;

        let mut urls = Vec::new();
        if state == VideoTaskState::Succeed {
            for c in &parsed.creations {
                if let Some(url) = c.url.as_deref().filter(|u| !u.trim().is_empty()) {
                    urls.push(url.to_string());
                    continue;
                }

                if let Some(cover) = c.cover_url.as_deref().filter(|u| !u.trim().is_empty()) {
                    urls.push(cover.to_string());
                }
            }
        }

        let message = match state {
            VideoTaskState::Failed => parsed.err_code.clone().or(parsed.payload.clone()),
            _ => None,
        };

        Ok(VideoTaskStatus {
            id: task_id.to_string(),
            state,
            message,
            video_urls: urls,
        })
    }
}
