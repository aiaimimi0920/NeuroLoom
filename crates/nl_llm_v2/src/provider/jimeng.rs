use serde::{Deserialize, Serialize};
use serde_json::json;
use async_trait::async_trait;
use anyhow::{Result, anyhow};

use crate::auth::traits::Authenticator;
use crate::provider::extension::{ProviderExtension, ModelInfo};
use crate::provider::extension::{VideoTaskState, VideoTaskStatus};
use crate::auth::providers::jimeng::JimengAuth;

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
        let endpoint = "https://visual.volcengineapi.com/?Action=CVProcess&Version=2022-08-31";
        
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
        
        let url_obj = reqwest::Url::parse(&endpoint).unwrap();
        let host = url_obj.host_str().unwrap();
        
        let body_bytes = serde_json::to_vec(&request_body).map_err(|e| anyhow!("Serialization error: {}", e))?;
        
        let mut builder = http.post(endpoint).json(&request_body);
        builder = auth.inject(builder)?;
        let mut req_obj = builder.build().unwrap();

        let mut ak = String::new();
        let mut sk = String::new();
        if let Some(auth_val) = req_obj.headers().get(reqwest::header::AUTHORIZATION) {
            let auth_str = auth_val.to_str().unwrap();
            if auth_str.starts_with("Bearer ") {
                let token = &auth_str[7..];
                let parts: Vec<&str> = token.split('|').collect();
                if parts.len() == 2 {
                    ak = parts[0].to_string();
                    sk = parts[1].to_string();
                }
            }
        }
        
        if ak.is_empty() || sk.is_empty() {
             return Err(anyhow!("Invalid Jimeng credentials. Expected 'AccessKey|SecretKey' in Bearer token."));
        }

        let jimeng_auth = JimengAuth::new(ak, sk);
        
        let mut headers = req_obj.headers_mut().clone();
        jimeng_auth.sign_request(
            "POST",
            host,
            "/",
            "Action=CVProcess&Version=2022-08-31",
            &mut headers,
            &body_bytes,
        )?;
        
        *req_obj.headers_mut() = headers;
        
        let response = http.execute(req_obj).await.map_err(|e| anyhow!("Network error: {}", e))?;
        let status = response.status();
        let res_text = response.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(anyhow!(
                "Jimeng API HTTP error ({}): {}",
                status, res_text
            ));
        }

        let task_resp: JimengSubmitResponse = serde_json::from_str(&res_text)
            .map_err(|e| anyhow!("Failed to parse response: {}, bodies: {}", e, res_text))?;

        if task_resp.code != 10000 {
            return Err(anyhow!(
                "Jimeng API error (Code {}): {}",
                task_resp.code, task_resp.message
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
        let endpoint = "https://visual.volcengineapi.com/?Action=CVSync2AsyncGetResult&Version=2022-08-31";
        
        let (req_key, raw_task_id) = if let Some((k, id)) = task_id.split_once(':') {
            (k.to_string(), id)
        } else {
            ("jimeng_t2v_v30".to_string(), task_id)
        };
        
        let request_body = json!({
            "req_key": req_key,
            "task_id": raw_task_id
        });
        
        let url_obj = reqwest::Url::parse(endpoint).unwrap();
        let host = url_obj.host_str().unwrap();
        let body_bytes = serde_json::to_vec(&request_body).unwrap();
        
        let mut builder = http.post(endpoint).json(&request_body);
        builder = auth.inject(builder)?;
        let mut req_obj = builder.build().unwrap();
        
        let mut ak = String::new();
        let mut sk = String::new();
        if let Some(auth_val) = req_obj.headers().get(reqwest::header::AUTHORIZATION) {
            let auth_str = auth_val.to_str().unwrap();
            if auth_str.starts_with("Bearer ") {
                let token = &auth_str[7..];
                let parts: Vec<&str> = token.split('|').collect();
                if parts.len() == 2 {
                    ak = parts[0].to_string();
                    sk = parts[1].to_string();
                }
            }
        }
        
        if ak.is_empty() || sk.is_empty() {
             return Err(anyhow!("Invalid Jimeng credentials in fetch task"));
        }

        let jimeng_auth = JimengAuth::new(ak, sk);
        let mut headers = req_obj.headers_mut().clone();
        jimeng_auth.sign_request(
            "POST",
            host,
            "/",
            "Action=CVSync2AsyncGetResult&Version=2022-08-31",
            &mut headers,
            &body_bytes,
        )?;
        *req_obj.headers_mut() = headers;

        let response = http.execute(req_obj).await.map_err(|e| anyhow!("Network error: {}", e))?;
        let status = response.status();
        let res_text = response.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(anyhow!(
                "Jimeng API fetch error ({}): {}",
                status, res_text
            ));
        }

        let task_resp: JimengFetchResponse = serde_json::from_str(&res_text)
            .map_err(|e| anyhow!("Parse error: {}", e))?;

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
