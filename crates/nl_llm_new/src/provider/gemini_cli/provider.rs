//! Gemini CLI Provider 实现
//!
//! 使用 Google Cloud Code (API Node.js CLI) 协议的 OAuth 凭据进行对话

use super::config::GeminiCliConfig;
use crate::auth::providers::gemini_cli::GeminiCliOAuth;
use crate::auth::{Auth, OAuthProvider};
use crate::provider::{GenericClient, Endpoint};
use crate::provider::gemini::protocol::CloudCodeProtocol;
use crate::generic_client;
use async_trait::async_trait;
use tokio::sync::Mutex;
use std::sync::Arc;

pub struct GeminiCliEndpoint {
    auth: Arc<Mutex<GeminiCliOAuth>>,
}

#[async_trait]
impl Endpoint for GeminiCliEndpoint {
    async fn pre_flight(&self) -> crate::Result<()> {
        let mut auth_guard = self.auth.lock().await;
        auth_guard.ensure_authenticated().await.map_err(|e| crate::Error::Auth(e.to_string()))?;
        Ok(())
    }

    fn url(&self, _model: &str, is_stream: bool) -> crate::Result<String> {
        if is_stream {
            Ok("https://cloudcode-pa.googleapis.com/v1internal:streamGenerateContent?alt=sse".to_string())
        } else {
            Ok("https://cloudcode-pa.googleapis.com/v1internal:generateContent".to_string())
        }
    }

    fn decorate_body(&self, mut body: serde_json::Value) -> serde_json::Value {
        // Here we can inject project id, since it was empty in the protocol compilation
        if let Ok(guard) = self.auth.try_lock() {
            if let Some(project) = guard.project_id() {
                if let Some(obj) = body.as_object_mut() {
                    obj.insert("project".to_string(), serde_json::Value::String(project.to_string()));
                }
            }
        }
        body
    }

    fn inject_auth(&self, req: reqwest::RequestBuilder) -> crate::Result<reqwest::RequestBuilder> {
        let token = {
            let guard = self.auth.try_lock().map_err(|_| crate::Error::Auth("Auth Mutex Failed".to_string()))?;
            guard.access_token().map(|s| s.to_string()).ok_or_else(|| crate::Error::Auth("No access token available".to_string()))?
        };

        let req = req
            .header("Authorization", format!("Bearer {}", token))
            .header("User-Agent", "google-api-nodejs-client/9.15.1")
            .header("X-Goog-Api-Client", "gl-node/22.17.0")
            .header("Client-Metadata", "ideType=IDE_UNSPECIFIED,platform=PLATFORM_UNSPECIFIED,pluginType=GEMINI");

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

pub type GeminiCliProvider = GenericClient<GeminiCliEndpoint, CloudCodeProtocol>;

impl GeminiCliProvider {
    /// 创建新的 Provider（需要外部传入 HTTP Client）
    ///
    /// 注意：根据设计规范，HTTP Client 应由外部统一管理，
    /// 避免每个 Provider 重复创建连接池。
    pub fn new(config: GeminiCliConfig, http: reqwest::Client) -> crate::Result<Self> {
        let auth_engine = GeminiCliOAuth::from_file(&config.token_path)
            .map_err(|e| crate::Error::Auth(e.to_string()))?;

        let auth_enum = Auth::OAuth {
            provider: OAuthProvider::GeminiCli,
            token_path: config.token_path.clone(),
        };

        let shared_auth = Arc::new(Mutex::new(auth_engine));

        Ok(generic_client! {
            id: "gemini_cli".to_string(),
            endpoint: GeminiCliEndpoint { auth: shared_auth },
            protocol: CloudCodeProtocol { default_model: config.model.clone() },
            auth: auth_enum,
            supported_models: vec![
                "gemini-2.5-flash".to_string(),
                "gemini-2.5-pro".to_string(),
                "gemini-2.0-flash".to_string(),
                "gemini-2.0-pro-exp-02-05".to_string(),
                "gemini-1.5-pro".to_string(),
                "gemini-1.5-flash".to_string(),
            ],
            http: http
        })
    }
}
