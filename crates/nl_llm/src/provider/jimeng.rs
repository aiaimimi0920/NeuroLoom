use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::providers::jimeng::JimengAuth;
use crate::auth::traits::Authenticator;
use crate::provider::extension::{ModelInfo, ProviderExtension};
use crate::provider::extension::{VideoTaskState, VideoTaskStatus};

const JIMENG_SUBMIT_ENDPOINT: &str =
    "https://visual.volcengineapi.com/?Action=CVProcess&Version=2022-08-31";
const JIMENG_FETCH_ENDPOINT: &str =
    "https://visual.volcengineapi.com/?Action=CVSync2AsyncGetResult&Version=2022-08-31";

// Volcengine Payload struct
#[derive(Serialize, Debug)]
struct JimengVideoRequest {
    req_key: String,
    prompt: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    image_urls: Option<Vec<String>>,

    // Default video params
    seed: i64,
    aspect_ratio: String,
    frames: i32,
}

#[derive(Deserialize, Debug)]
struct JimengSubmitResponse {
    code: i32,
    message: String,
    #[allow(dead_code)]
    request_id: String,
    data: Option<JimengSubmitData>,
}

#[derive(Deserialize, Debug)]
struct JimengSubmitData {
    task_id: String,
}

#[derive(Deserialize, Debug)]
struct JimengFetchResponse {
    code: i32,
    message: String,
    #[allow(dead_code)]
    request_id: String,
    data: Option<JimengFetchData>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct JimengFetchData {
    status: String,
    video_url: Option<String>,
    resp_data: Option<String>,
}

pub struct JimengExtension;

impl JimengExtension {
    pub fn new() -> Self {
        Self
    }

    fn parse_credentials(
        auth_header: Option<&reqwest::header::HeaderValue>,
    ) -> Result<(String, String)> {
        let auth_str = auth_header
            .ok_or_else(|| anyhow!("Missing Jimeng credentials in Authorization header"))?
            .to_str()
            .map_err(|e| anyhow!("Invalid Authorization header: {}", e))?;

        if !auth_str.starts_with("Bearer ") {
            return Err(anyhow!(
                "Invalid Jimeng credentials format. Expected Bearer token with 'AccessKey|SecretKey'."
            ));
        }

        let token = &auth_str[7..];
        let (ak, sk) = token.split_once('|').ok_or_else(|| {
            anyhow!("Invalid Jimeng credentials. Expected 'AccessKey|SecretKey' in Bearer token.")
        })?;

        if ak.is_empty() || sk.is_empty() {
            return Err(anyhow!(
                "Invalid Jimeng credentials. AccessKey or SecretKey is empty."
            ));
        }

        Ok((ak.to_string(), sk.to_string()))
    }

    fn sign_request(
        req_obj: &mut reqwest::Request,
        endpoint: &str,
        query: &str,
        body_bytes: &[u8],
    ) -> Result<()> {
        let host = reqwest::Url::parse(endpoint)
            .ok()
            .and_then(|url| url.host_str().map(str::to_string))
            .ok_or_else(|| anyhow!("Invalid Jimeng endpoint: {}", endpoint))?;

        let (ak, sk) =
            Self::parse_credentials(req_obj.headers().get(reqwest::header::AUTHORIZATION))?;
        let jimeng_auth = JimengAuth::new(ak, sk);

        let mut headers = req_obj.headers().clone();
        jimeng_auth.sign_request("POST", &host, "/", query, &mut headers, body_bytes)?;
        *req_obj.headers_mut() = headers;

        Ok(())
    }
}

#[async_trait]
impl ProviderExtension for JimengExtension {
    fn id(&self) -> &str {
        "jimeng"
    }

    async fn list_models(
        &self,
        _http: &reqwest::Client,
        _auth: &mut dyn Authenticator,
    ) -> Result<Vec<ModelInfo>> {
        Ok(vec![
            ModelInfo {
                id: "jimeng-v2.0".to_string(),
                description: "Jimeng v2".to_string(),
            },
            ModelInfo {
                id: "jimeng-v3.0".to_string(),
                description: "Jimeng v3".to_string(),
            },
            ModelInfo {
                id: "jimeng-v3.0-pro".to_string(),
                description: "Jimeng v3 Pro".to_string(),
            },
        ])
    }

    async fn submit_video_task(
        &self,
        http: &reqwest::Client,
        auth: &mut dyn Authenticator,
        req: &crate::primitive::PrimitiveRequest,
    ) -> Result<String> {
        let mut prompt = String::new();
        let mut image_url: Option<String> = None;

        for msg in &req.messages {
            for content in &msg.content {
                match content {
                    crate::primitive::PrimitiveContent::Text { text } => {
                        if !prompt.is_empty() {
                            prompt.push('\n');
                        }
                        prompt.push_str(text);
                    }
                    crate::primitive::PrimitiveContent::Image { url, .. } => {
                        image_url = Some(url.clone());
                    }
                    _ => {}
                }
            }
        }

        let mut req_key = req.model.clone();

        if image_url.is_some() {
            req_key = match req_key.as_str() {
                "jimeng_t2v_v30" => "jimeng_i2v_first_v30".to_string(),
                _ => req_key,
            };
        }

        let request_body = JimengVideoRequest {
            req_key,
            prompt,
            image_urls: image_url.map(|url| vec![url]),
            seed: -1,
            aspect_ratio: "16:9".to_string(),
            frames: 121,
        };

        let body_bytes =
            serde_json::to_vec(&request_body).map_err(|e| anyhow!("Serialization error: {}", e))?;

        let mut builder = http.post(JIMENG_SUBMIT_ENDPOINT).json(&request_body);
        builder = auth.inject(builder)?;
        let mut req_obj = builder
            .build()
            .map_err(|e| anyhow!("Failed to build Jimeng submit request: {}", e))?;
        Self::sign_request(
            &mut req_obj,
            JIMENG_SUBMIT_ENDPOINT,
            "Action=CVProcess&Version=2022-08-31",
            &body_bytes,
        )?;

        let response = http
            .execute(req_obj)
            .await
            .map_err(|e| anyhow!("Network error: {}", e))?;
        let status = response.status();
        let res_text = response.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(anyhow!("Jimeng API HTTP error ({}): {}", status, res_text));
        }

        let task_resp: JimengSubmitResponse = serde_json::from_str(&res_text)
            .map_err(|e| anyhow!("Failed to parse response: {}, bodies: {}", e, res_text))?;

        if task_resp.code != 10000 {
            return Err(anyhow!(
                "Jimeng API error (Code {}): {}",
                task_resp.code,
                task_resp.message
            ));
        }

        if let Some(data) = task_resp.data {
            Ok(format!("{}:{}", req.model, data.task_id))
        } else {
            Err(anyhow!("Jimeng API error: missing task_id in response"))
        }
    }

    async fn fetch_video_task(
        &self,
        http: &reqwest::Client,
        auth: &mut dyn Authenticator,
        task_id: &str,
    ) -> Result<VideoTaskStatus> {
        let (req_key, raw_task_id) = if let Some((k, id)) = task_id.split_once(':') {
            (k.to_string(), id)
        } else {
            ("jimeng_t2v_v30".to_string(), task_id)
        };

        let request_body = json!({
            "req_key": req_key,
            "task_id": raw_task_id
        });

        let body_bytes =
            serde_json::to_vec(&request_body).map_err(|e| anyhow!("Serialization error: {}", e))?;

        let mut builder = http.post(JIMENG_FETCH_ENDPOINT).json(&request_body);
        builder = auth.inject(builder)?;
        let mut req_obj = builder
            .build()
            .map_err(|e| anyhow!("Failed to build Jimeng fetch request: {}", e))?;
        Self::sign_request(
            &mut req_obj,
            JIMENG_FETCH_ENDPOINT,
            "Action=CVSync2AsyncGetResult&Version=2022-08-31",
            &body_bytes,
        )?;

        let response = http
            .execute(req_obj)
            .await
            .map_err(|e| anyhow!("Network error: {}", e))?;
        let status = response.status();
        let res_text = response.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(anyhow!("Jimeng API fetch error ({}): {}", status, res_text));
        }

        let task_resp: JimengFetchResponse =
            serde_json::from_str(&res_text).map_err(|e| anyhow!("Parse error: {}", e))?;

        if task_resp.code != 10000 {
            return Ok(VideoTaskStatus {
                id: raw_task_id.to_string(),
                state: VideoTaskState::Failed,
                video_urls: vec![],
                message: Some(task_resp.message),
            });
        }

        if let Some(data) = task_resp.data {
            let state = match data.status.as_str() {
                "in_queue" | "processing" => VideoTaskState::Processing,
                "done" => VideoTaskState::Succeed,
                "failed" => VideoTaskState::Failed,
                _ => VideoTaskState::Failed,
            };

            Ok(VideoTaskStatus {
                id: raw_task_id.to_string(),
                state,
                message: None,
                video_urls: data.video_url.map(|u| vec![u]).unwrap_or_default(),
            })
        } else {
            Err(anyhow!("Missing task data in Jimeng fetch response"))
        }
    }
}
