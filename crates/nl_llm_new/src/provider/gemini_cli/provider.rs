//! Gemini CLI Provider 实现
//!
//! 使用 Google Cloud Code (API Node.js CLI) 协议的 OAuth 凭据进行对话

use super::config::GeminiCliConfig;
use crate::auth::providers::gemini_cli::GeminiCliOAuth;
use crate::auth::{Auth, OAuthProvider};
use crate::primitive::PrimitiveRequest;
use crate::provider::{BoxStream, LlmChunk, LlmProvider, LlmResponse, StopReason, Usage};
use async_trait::async_trait;
use std::time::Duration;
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct GeminiCliProvider {
    config: GeminiCliConfig,
    auth: Mutex<GeminiCliOAuth>,
    http: reqwest::Client,
    auth_enum: Auth,
}

impl GeminiCliProvider {
    pub fn new(config: GeminiCliConfig) -> crate::Result<Self> {
        let auth_engine = GeminiCliOAuth::from_file(&config.token_path)
            .map_err(|e| crate::Error::Auth(e.to_string()))?;

        let auth_enum = Auth::OAuth {
            provider: OAuthProvider::GeminiCli,
            token_path: config.token_path.clone(),
        };

        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        Ok(Self {
            config,
            auth: Mutex::new(auth_engine),
            http,
            auth_enum,
        })
    }

    async fn get_access_token_and_project(&self) -> crate::Result<(String, String)> {
        let mut auth_guard = self.auth.lock().await;
        auth_guard.ensure_authenticated().await.map_err(|e| crate::Error::Auth(e.to_string()))?;
        
        let token = auth_guard.access_token().ok_or_else(|| crate::Error::Auth("No access token available".to_string()))?.to_string();
        let project_id = auth_guard.token.as_ref()
            .and_then(|t| t.project_id.clone())
            .unwrap_or_else(|| "fallback-project-id".to_string());
            
        Ok((token, project_id))
    }
}

#[async_trait]
impl LlmProvider for GeminiCliProvider {
    fn id(&self) -> &str {
        "gemini_cli"
    }

    fn auth(&self) -> &Auth {
        &self.auth_enum
    }

    fn supported_models(&self) -> &[&str] {
        &[
            "gemini-2.5-flash",
            "gemini-2.5-pro",
            "gemini-2.0-flash",
            "gemini-2.0-pro-exp-02-05",
            "gemini-1.5-pro",
            "gemini-1.5-flash",
        ]
    }

    fn compile(&self, primitive: &PrimitiveRequest) -> serde_json::Value {
        let mut req = primitive.clone();
        if req.model.is_empty() {
            req.model = self.config.model.clone();
        }

        let compiler = crate::provider::gemini::compiler::GeminiCompiler;
        let inner_request = compiler.compile(&req);
        
        let mut request_payload = serde_json::json!({
            "sessionId": format!("-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() & 0x7FFFFFFFFFFFFFFF)
        });
        
        if let Some(contents) = inner_request.get("contents") {
            request_payload["contents"] = contents.clone();
        } else {
            request_payload["contents"] = serde_json::json!([]);
        }
        
        if let Some(sys) = inner_request.get("systemInstruction") {
            if !sys.is_null() {
                request_payload["systemInstruction"] = sys.clone();
            }
        }

        serde_json::json!({
            "model": req.model,
            "userAgent": "gemini-cli",
            "requestType": "agent",
            "project": "", // Will be filled dynamically in complete/stream
            "requestId": format!("agent-{}", Uuid::new_v4()),
            "request": request_payload
        })
    }

    async fn complete(&self, mut body: serde_json::Value) -> crate::Result<LlmResponse> {
        let (token, project_id) = self.get_access_token_and_project().await?;
        
        if let Some(obj) = body.as_object_mut() {
            obj.insert("project".to_string(), serde_json::Value::String(project_id));
        }

        let url = "https://cloudcode-pa.googleapis.com/v1internal:generateContent";
        let resp = self.http.post(url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .header("User-Agent", "google-api-nodejs-client/9.15.1")
            .header("X-Goog-Api-Client", "gl-node/22.17.0")
            .header("Client-Metadata", "ideType=IDE_UNSPECIFIED,platform=PLATFORM_UNSPECIFIED,pluginType=GEMINI")
            .json(&body)
            .send()
            .await
            .map_err(|e| crate::Error::Http(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(crate::Error::Provider(format!(
                "gemini-cli generate Content failed: [{}] {}",
                status, text
            )));
        }

        let text = resp.text().await.unwrap_or_default();
        let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| crate::Error::Json(e))?;
        
        let candidates = v.get("response").and_then(|r| r.get("candidates")).or_else(|| v.get("candidates"));
        let content = candidates
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("content"))
            .and_then(|c| c.get("parts"))
            .and_then(|p| p.get(0))
            .and_then(|p| p.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();

        Ok(LlmResponse {
            content,
            tool_calls: Vec::new(),
            usage: Usage::default(),
            stop_reason: StopReason::EndTurn,
        })
    }

    async fn stream(
        &self,
        mut body: serde_json::Value,
    ) -> crate::Result<BoxStream<'_, crate::Result<LlmChunk>>> {
        let (token, project_id) = self.get_access_token_and_project().await?;
        
        if let Some(obj) = body.as_object_mut() {
            obj.insert("project".to_string(), serde_json::Value::String(project_id));
        }

        let url = "https://cloudcode-pa.googleapis.com/v1internal:streamGenerateContent?alt=sse";
        let resp = self.http.post(url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .header("User-Agent", "google-api-nodejs-client/9.15.1")
            .header("X-Goog-Api-Client", "gl-node/22.17.0")
            .header("Client-Metadata", "ideType=IDE_UNSPECIFIED,platform=PLATFORM_UNSPECIFIED,pluginType=GEMINI")
            .json(&body)
            .send()
            .await
            .map_err(|e| crate::Error::Http(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(crate::Error::Provider(format!(
                "gemini-cli stream failed: [{}] {}",
                status, text
            )));
        }

        use futures::StreamExt;
        let stream = async_stream::stream! {
            let mut byte_stream = resp.bytes_stream();
            let mut buffer = String::new();

            while let Some(chunk_res) = byte_stream.next().await {
                let bytes = match chunk_res {
                    Ok(b) => b,
                    Err(e) => {
                        yield Err(crate::Error::Http(e.to_string()));
                        continue;
                    }
                };
                
                buffer.push_str(&String::from_utf8_lossy(&bytes));

                while let Some(pos) = buffer.find("\r\n\r\n").or_else(|| buffer.find("\n\n")) {
                    let offset = if buffer[pos..].starts_with("\r\n\r\n") { 4 } else { 2 };
                    let event = buffer[..pos].to_string();
                    buffer = buffer[pos + offset..].to_string();

                    for line in event.lines() {
                        if let Some(data) = line.strip_prefix("data: ") {
                            if data == "[DONE]" {
                                return;
                            }
                            if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
                                let candidates = v.get("response").and_then(|r| r.get("candidates")).or_else(|| v.get("candidates"));
                                if let Some(content) = candidates
                                    .and_then(|c| c.get(0).or_else(|| c.as_array().and_then(|a| a.get(0))))
                                    .and_then(|c| c.get("content"))
                                    .and_then(|c| c.get("parts"))
                                    .and_then(|p| p.get(0).or_else(|| p.as_array().and_then(|a| a.get(0))))
                                    .and_then(|p| p.get("text"))
                                    .and_then(|t| t.as_str())
                                {
                                    if !content.is_empty() {
                                        yield Ok(LlmChunk {
                                            delta: crate::provider::ChunkDelta::Text(content.to_string()),
                                            usage: None,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }

    fn needs_refresh(&self) -> bool {
        if let Ok(guard) = self.auth.try_lock() {
            guard.token.as_ref().map_or(true, |t| {
                t.expires_at <= chrono::Utc::now() + chrono::Duration::seconds(300)
            })
        } else {
            false
        }
    }

    async fn refresh_auth(&mut self) -> crate::Result<()> {
        let mut auth_guard = self.auth.lock().await;
        auth_guard.ensure_authenticated().await.map_err(|e| crate::Error::Auth(e.to_string()))?;
        Ok(())
    }
}
