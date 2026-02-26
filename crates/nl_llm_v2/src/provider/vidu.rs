use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::auth::traits::Authenticator;
use crate::primitive::{PrimitiveContent, PrimitiveRequest};
use crate::provider::extension::{ModelInfo, ProviderExtension, VideoTaskState, VideoTaskStatus};

fn env_opt(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|s| !s.trim().is_empty())
}

fn env_bool(name: &str) -> Option<bool> {
    env_opt(name).and_then(|v| match v.trim().to_lowercase().as_str() {
        "1" | "true" | "yes" | "y" | "on" => Some(true),
        "0" | "false" | "no" | "n" | "off" => Some(false),
        _ => None,
    })
}

fn vidu_base_url_candidates() -> Vec<String> {
    // 兼容不同文档/区域：
    // - new-api 默认使用 https://api.vidu.cn
    // - 官方文档（platform.vidu.com）常见 https://api.vidu.com
    //
    // 允许通过环境变量覆盖（优先级从高到低）：
    // - VIDU_API_BASE_URL
    // - VIDU_BASE_URL
    //
    // 若未显式指定，则按候选列表依次尝试（用于处理“域名不匹配导致 401/403”）。
    if let Some(v) = env_opt("VIDU_API_BASE_URL").or_else(|| env_opt("VIDU_BASE_URL")) {
        return vec![v];
    }

    vec![
        "https://api.vidu.cn".to_string(),
        "https://api.vidu.com".to_string(),
    ]
}

fn default_duration_for_model(model: &str) -> i32 {
    // 经验默认：
    // - Vidu Q1 常见默认 5s
    // - Vidu 2.0 / 1.5 常见默认 4s（很多计费档位是 4s/8s）
    //
    // 若用户通过 extra 指定 duration，会覆盖此默认。
    let m = model.trim().to_lowercase();
    if m == "viduq1" {
        5
    } else if m == "vidu2.0" || m == "vidu1.5" {
        4
    } else {
        5
    }
}

fn default_resolution_for_model(model: &str) -> String {
    // 经验默认：
    // - Q1 常见为 1080p
    // - 其他模型默认 720p（更通用/更省点数）
    let m = model.trim().to_lowercase();
    if m == "viduq1" {
        "1080p".to_string()
    } else {
        "720p".to_string()
    }
}

/// Vidu 官方 API：任务提交的动作类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ViduAction {
    Text2Video,
    Img2Video,
    StartEnd2Video,
    Reference2Video,
}

impl ViduAction {
    fn path(&self) -> &'static str {
        match self {
            ViduAction::Img2Video => "/ent/v2/img2video",
            ViduAction::StartEnd2Video => "/ent/v2/start-end2video",
            ViduAction::Reference2Video => "/ent/v2/reference2video",
            ViduAction::Text2Video => "/ent/v2/text2video",
        }
    }
}

fn pick_action(req: &PrimitiveRequest, images_len: usize) -> ViduAction {
    // 允许用户通过 extra 显式指定动作（优先于自动推断）
    // 支持：text2video / img2video / start-end2video / reference2video
    if let Some(v) = req.extra.get("vidu_action").and_then(|v| v.as_str()) {
        return match v {
            "text2video" => ViduAction::Text2Video,
            "img2video" => ViduAction::Img2Video,
            "start-end2video" => ViduAction::StartEnd2Video,
            "reference2video" => ViduAction::Reference2Video,
            _ => {
                // 未知值则回退到自动
                infer_action(images_len)
            }
        };
    }

    infer_action(images_len)
}

fn infer_action(images_len: usize) -> ViduAction {
    match images_len {
        0 => ViduAction::Text2Video,
        1 => ViduAction::Img2Video,
        2 => ViduAction::StartEnd2Video,
        _ => ViduAction::Reference2Video,
    }
}

fn collect_prompt_and_images(req: &PrimitiveRequest) -> (String, Vec<String>) {
    let mut prompt = String::new();
    let mut images = Vec::new();

    for msg in &req.messages {
        for c in &msg.content {
            match c {
                PrimitiveContent::Text { text } => {
                    if !prompt.is_empty() {
                        prompt.push('\n');
                    }
                    prompt.push_str(text);
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

    (prompt, images)
}

fn extra_string(req: &PrimitiveRequest, key: &str) -> Option<String> {
    req.extra
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn extra_i32(req: &PrimitiveRequest, key: &str) -> Option<i32> {
    req.extra.get(key).and_then(|v| {
        if let Some(n) = v.as_i64() {
            Some(n as i32)
        } else if let Some(n) = v.as_u64() {
            Some(n as i32)
        } else {
            None
        }
    })
}

fn extra_bool(req: &PrimitiveRequest, key: &str) -> Option<bool> {
    req.extra.get(key).and_then(|v| v.as_bool())
}

// ============================
// Request / Response structures
// ============================

#[derive(Debug, Serialize)]
struct ViduRequestPayload {
    model: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
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

    /// Off-peak mode（低谷模式，通常可减少点数消耗）
    ///
    /// 来自官方更新公告：`off_peak` 参数适用于 img2video/reference2video/text2video/start-end2video。
    #[serde(skip_serializing_if = "Option::is_none")]
    off_peak: Option<bool>,

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
    #[allow(dead_code)]
    id: String,
    url: String,
    #[allow(dead_code)]
    cover_url: String,
}

#[derive(Debug, Deserialize)]
struct ViduFetchResponse {
    state: String,
    #[serde(default)]
    err_code: String,
    #[allow(dead_code)]
    credits: Option<i32>,
    #[allow(dead_code)]
    payload: Option<String>,
    #[serde(default)]
    creations: Vec<ViduTaskCreation>,
}

pub struct ViduExtension;

impl ViduExtension {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ViduExtension {
    fn default() -> Self {
        Self::new()
    }
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
    ) -> anyhow::Result<Vec<ModelInfo>> {
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
    ) -> anyhow::Result<String> {
        let (prompt, images) = collect_prompt_and_images(req);
        let action = pick_action(req, images.len());

        // model 默认从 primitive.model 取（外层已做别名 resolve）
        let mut model = req.model.clone();
        if model.trim().is_empty() {
            model = "viduq1".to_string();
        }

        // new-api 规则：reference2video 且模型含 viduq2 时，强制使用纯 "viduq2"
        // 以避免携带 pro/turbo 等后缀导致不兼容。
        if action == ViduAction::Reference2Video && model.contains("viduq2") {
            model = "viduq2".to_string();
        }

        // 参数默认值：
        // 1) extra 显式传入优先
        // 2) 其次读取环境变量（便于 examples/test.bat 快速测试）
        // 3) 最后用“按模型经验默认”兜底
        let duration = extra_i32(req, "duration")
            .or_else(|| env_opt("VIDU_DURATION").and_then(|v| v.parse::<i32>().ok()))
            .or_else(|| Some(default_duration_for_model(&model)));

        let resolution = extra_string(req, "resolution")
            .or_else(|| env_opt("VIDU_RESOLUTION"))
            .or_else(|| Some(default_resolution_for_model(&model)));

        let movement_amplitude = extra_string(req, "movement_amplitude")
            .or_else(|| env_opt("VIDU_MOVEMENT_AMPLITUDE"))
            .or_else(|| Some("auto".to_string()));

        let bgm = extra_bool(req, "bgm")
            .or_else(|| env_bool("VIDU_BGM"))
            .or_else(|| Some(false));

        let off_peak = extra_bool(req, "off_peak")
            .or_else(|| env_bool("VIDU_OFF_PEAK"));

        let seed = extra_i32(req, "seed")
            .or_else(|| env_opt("VIDU_SEED").and_then(|v| v.parse::<i32>().ok()));

        // callback_url / payload 允许通过 env 覆盖，便于调试回调
        let callback_url = extra_string(req, "callback_url").or_else(|| env_opt("VIDU_CALLBACK_URL"));
        let payload = extra_string(req, "payload").or_else(|| env_opt("VIDU_PAYLOAD"));

        let prompt_opt = if prompt.trim().is_empty() {
            None
        } else {
            Some(prompt)
        };

        let base_urls = vidu_base_url_candidates();
        let mut last_err: Option<anyhow::Error> = None;

        for (i, base_url) in base_urls.iter().enumerate() {
            let body = ViduRequestPayload {
                model: model.clone(),
                images: images.clone(),
                prompt: prompt_opt.clone(),
                duration,
                seed,
                resolution: resolution.clone(),
                movement_amplitude: movement_amplitude.clone(),
                bgm,
                off_peak,
                payload: payload.clone(),
                callback_url: callback_url.clone(),
            };

            let url = format!("{}{}", base_url.trim_end_matches('/'), action.path());

            let mut rb = http
                .post(&url)
                .header("Accept", "application/json")
                .header("Content-Type", "application/json")
                .json(&body);
            rb = auth.inject(rb)?;

            let resp = rb.send().await?;
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();

            if !status.is_success() {
                // 401/403：优先尝试下一个候选域名（若用户未显式指定 base_url）。
                let err = anyhow::anyhow!(
                    "Vidu submit HTTP error ({}): {} | base_url={}",
                    status,
                    if text.trim().is_empty() { "<empty body>" } else { text.as_str() },
                    base_url
                );
                last_err = Some(err);

                let should_try_next = (status.as_u16() == 401 || status.as_u16() == 403)
                    && i + 1 < base_urls.len();

                if should_try_next {
                    continue;
                }

                return Err(last_err.unwrap());
            }

            let parsed: ViduSubmitResponse = serde_json::from_str(&text).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to parse Vidu submit response: {} | base_url={} | body={}",
                    e,
                    base_url,
                    text
                )
            })?;

            if parsed.state == "failed" {
                return Err(anyhow::anyhow!(
                    "Vidu task failed on submit: task_id={} | base_url={}",
                    parsed.task_id,
                    base_url
                ));
            }

            return Ok(parsed.task_id);
        }

        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("Vidu submit failed")))
    }

    async fn fetch_video_task(
        &self,
        http: &reqwest::Client,
        auth: &mut dyn Authenticator,
        task_id: &str,
    ) -> anyhow::Result<VideoTaskStatus> {
        let base_urls = vidu_base_url_candidates();
        let mut last_err: Option<anyhow::Error> = None;

        let mut parsed: Option<ViduFetchResponse> = None;
        let mut used_base_url: Option<String> = None;

        for (i, base_url) in base_urls.iter().enumerate() {
            let url = format!(
                "{}/ent/v2/tasks/{}/creations",
                base_url.trim_end_matches('/'),
                task_id
            );

            let mut rb = http
                .get(&url)
                .header("Accept", "application/json");
            rb = auth.inject(rb)?;

            let resp = rb.send().await?;
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();

            if !status.is_success() {
                let err = anyhow::anyhow!(
                    "Vidu fetch HTTP error ({}): {} | base_url={}",
                    status,
                    if text.trim().is_empty() { "<empty body>" } else { text.as_str() },
                    base_url
                );
                last_err = Some(err);

                let should_try_next = (status.as_u16() == 401 || status.as_u16() == 403)
                    && i + 1 < base_urls.len();

                if should_try_next {
                    continue;
                }

                return Err(last_err.unwrap());
            }

            let p: ViduFetchResponse = serde_json::from_str(&text).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to parse Vidu fetch response: {} | base_url={} | body={}",
                    e,
                    base_url,
                    text
                )
            })?;

            parsed = Some(p);
            used_base_url = Some(base_url.clone());
            break;
        }

        let parsed = parsed.ok_or_else(|| last_err.unwrap_or_else(|| anyhow::anyhow!("Vidu fetch failed")))?;
        let _used_base_url = used_base_url.unwrap_or_else(|| "<unknown>".to_string());

        let state = match parsed.state.as_str() {
            "created" | "queueing" => VideoTaskState::Submitted,
            "processing" => VideoTaskState::Processing,
            "success" => VideoTaskState::Succeed,
            "failed" => VideoTaskState::Failed,
            _ => VideoTaskState::Processing,
        };

        let mut urls = Vec::new();
        for c in &parsed.creations {
            if !c.url.trim().is_empty() {
                urls.push(c.url.clone());
            }
        }

        let message = if state == VideoTaskState::Failed {
            if parsed.err_code.trim().is_empty() {
                Some("Vidu task failed".to_string())
            } else {
                Some(parsed.err_code)
            }
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
