//! Antigravity Provider 实现
//!
//! 使用 Google OAuth 认证，调用 Cloud Code PA API

use super::config::AntigravityConfig;
use crate::auth::providers::antigravity::AntigravityOAuth;
use crate::auth::{Auth, OAuthProvider};
use crate::provider::{GenericClient, Endpoint};
use crate::provider::gemini::protocol::CloudCodeProtocol;
use crate::generic_client;
use async_trait::async_trait;
use tokio::sync::Mutex;
use std::sync::Arc;

/// API 端点
const BASE_URL: &str = "https://cloudcode-pa.googleapis.com";
const API_VERSION: &str = "v1internal";

pub struct AntigravityEndpoint {
    auth: Arc<Mutex<AntigravityOAuth>>,
}

impl AntigravityEndpoint {
    /// 生成 Project ID
    fn generate_project_id() -> String {
        let adjectives = ["useful", "bright", "swift", "calm", "bold"];
        let nouns = ["fuze", "wave", "spark", "flow", "core"];
        let uid = uuid::Uuid::new_v4().to_string();
        let random_part = &uid[..5];
        let nanos = chrono::Utc::now().timestamp_subsec_nanos() as usize;
        let adj = adjectives[nanos % adjectives.len()];
        let noun = nouns[(nanos / 2) % nouns.len()];
        format!("{}-{}-{}", adj, noun, random_part)
    }
}

#[async_trait]
impl Endpoint for AntigravityEndpoint {
    async fn pre_flight(&self) -> crate::Result<()> {
        let mut auth_guard = self.auth.lock().await;
        auth_guard
            .ensure_authenticated()
            .await
            .map_err(|e| crate::Error::Auth(e.to_string()))
    }

    fn url(&self, _model: &str, is_stream: bool) -> crate::Result<String> {
        let action = if is_stream { "streamGenerateContent?alt=sse" } else { "generateContent" };
        Ok(format!("{}/{}:{}", BASE_URL, API_VERSION, action))
    }

    fn decorate_body(&self, mut body: serde_json::Value) -> serde_json::Value {
        let project = if let Ok(guard) = self.auth.try_lock() {
            if let Some(pid) = guard.project_id() {
                if !pid.is_empty() {
                    pid.to_string()
                } else {
                    Self::generate_project_id()
                }
            } else {
                Self::generate_project_id()
            }
        } else {
            Self::generate_project_id()
        };

        if let Some(obj) = body.as_object_mut() {
            obj.insert("project".to_string(), serde_json::Value::String(project));
        }

        body
    }

    fn inject_auth(&self, mut req: reqwest::RequestBuilder) -> crate::Result<reqwest::RequestBuilder> {
        let access_token = {
            let guard = self.auth.try_lock().map_err(|_| crate::Error::Auth("Auth Mutex blocked".to_string()))?;
            guard.access_token().map(|s| s.to_string()).ok_or_else(|| crate::Error::Auth("No access token".to_string()))?
        };

        req = req
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .header("User-Agent", "google-cloud-sdk gcloud/0.0.0.dev")
            .header("X-Goog-Api-Client", "gl-python/3.12.0")
            .header("Client-Metadata", r#"{"ideType":"IDE_UNSPECIFIED","platform":"PLATFORM_UNSPECIFIED","pluginType":"GEMINI"}"#);

        Ok(req)
    }

    fn needs_refresh(&self) -> bool {
        if let Ok(guard) = self.auth.try_lock() {
            guard.needs_refresh()
        } else {
            false
        }
    }

    async fn refresh_auth(&self) -> crate::Result<()> {
        let mut auth_guard = self.auth.lock().await;
        auth_guard.ensure_authenticated().await.map_err(|e| crate::Error::Auth(e.to_string()))?;
        Ok(())
    }
}

pub type AntigravityProvider = GenericClient<AntigravityEndpoint, CloudCodeProtocol>;

impl AntigravityProvider {
    /// 创建新的 Provider（需要外部传入 HTTP Client）
    ///
    /// 注意：根据设计规范，HTTP Client 应由外部统一管理，
    /// 避免每个 Provider 重复创建连接池。
    pub fn new(config: AntigravityConfig, http: reqwest::Client) -> Self {
        let auth = AntigravityOAuth::from_file(&config.token_path)
            .expect("Failed to load AntigravityOAuth");

        let auth_enum = Auth::OAuth {
            provider: OAuthProvider::Antigravity,
            token_path: config.token_path.clone(),
        };

        let shared_auth = Arc::new(Mutex::new(auth));

        generic_client! {
            id: "antigravity".to_string(),
            endpoint: AntigravityEndpoint { auth: shared_auth },
            protocol: CloudCodeProtocol { default_model: config.model.clone() },
            auth: auth_enum,
            supported_models: vec![
                "gemini-2.5-flash".to_string(),
                "gemini-2.5-pro".to_string(),
                "gemini-1.5-pro".to_string(),
            ],
            http: http
        }
    }

    /// 使用默认配置创建 Provider
    pub fn with_default_config(model: String, http: reqwest::Client) -> Self {
        Self::new(AntigravityConfig::with_default_path(model), http)
    }
}
