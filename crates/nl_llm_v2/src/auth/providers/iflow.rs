use std::path::{Path, PathBuf};
use async_trait::async_trait;
use reqwest::{Client, RequestBuilder, header::{HeaderMap, HeaderValue}};
use serde::Deserialize;

use crate::auth::traits::Authenticator;
use crate::auth::types::{TokenStatus, TokenStorage};
use crate::site::context::AuthType;

/// iFlow API Key 响应结构
#[derive(Debug, Deserialize)]
struct IFlowApiKeyResponse {
    #[serde(default)]
    success: bool,
    #[serde(default)]
    data: Option<IFlowKeyData>,
}

#[derive(Debug, Deserialize)]
struct IFlowKeyData {
    name: String,
    #[serde(rename = "apiKey", default)]
    api_key: String,
}

/// iFlow Cookie 认证器（使用 Cookie 换取临时的 API Key 并在 Header 中注入）
pub struct IFlowAuth {
    token: Option<TokenStorage>,
    cookie_str: String,
    http: Client,
    cache_path: Option<PathBuf>,
}

impl IFlowAuth {
    pub fn new(cookie: impl Into<String>) -> Self {
        Self {
            token: None,
            cookie_str: Self::extract_bx_auth(&cookie.into()),
            http: Client::new(),
            cache_path: None,
        }
    }

    pub fn with_cache(mut self, path: impl AsRef<Path>) -> Self {
        self.cache_path = Some(path.as_ref().to_path_buf());
        if let Some(p) = &self.cache_path {
            if p.exists() {
                if let Ok(content) = std::fs::read_to_string(p) {
                    if let Ok(token) = serde_json::from_str::<TokenStorage>(&content) {
                        self.token = Some(token);
                    }
                }
            }
        }
        self
    }

    fn extract_bx_auth(cookie: &str) -> String {
        for part in cookie.split(';') {
            let part = part.trim();
            if part.starts_with("BXAuth=") {
                return format!("{};", part);
            }
        }
        if !cookie.is_empty() && !cookie.ends_with(';') {
            format!("{};", cookie)
        } else {
            cookie.to_string()
        }
    }

    fn build_headers(&self, is_post: bool) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Cookie",
            HeaderValue::from_str(&self.cookie_str).unwrap_or_else(|_| HeaderValue::from_static("")),
        );
        headers.insert(
            "User-Agent",
            HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0.0.0"),
        );
        if is_post {
            headers.insert("Content-Type", HeaderValue::from_static("application/json"));
            headers.insert("Origin", HeaderValue::from_static("https://platform.iflow.cn"));
            headers.insert("Referer", HeaderValue::from_static("https://platform.iflow.cn/"));
        }
        headers
    }
}

#[async_trait]
impl Authenticator for IFlowAuth {
    fn id(&self) -> &str {
        "iflow_cookie"
    }

    fn is_authenticated(&self) -> bool {
        self.token.is_some()
    }

    fn needs_refresh(&self) -> bool {
        self.token.as_ref().map_or(true, |t| {
            matches!(t.status(7 * 24 * 3600), TokenStatus::Expired | TokenStatus::ExpiringSoon)
        })
    }

    async fn refresh(&mut self) -> anyhow::Result<()> {
        if self.cookie_str.is_empty() {
            return Err(anyhow::anyhow!("Missing cookie for iFlow auth"));
        }

        // STEP 1: GET 获取用户信息与 name 
        let get_resp = self.http.get("https://platform.iflow.cn/api/openapi/apikey")
            .headers(self.build_headers(false))
            .send()
            .await?;

        if !get_resp.status().is_success() {
            return Err(anyhow::anyhow!("GET apikey failed ({}): {}", get_resp.status(), get_resp.text().await.unwrap_or_default()));
        }

        let get_data: IFlowApiKeyResponse = get_resp.json().await?;
        if !get_data.success {
            return Err(anyhow::anyhow!("GET apikey returned success=false"));
        }
        let data = get_data.data.ok_or_else(|| anyhow::anyhow!("Missing GET response payload"))?;

        // STEP 2: POST 刷新以获取真正的 apiKey
        let post_body = serde_json::json!({ "name": data.name });
        let post_resp = self.http.post("https://platform.iflow.cn/api/openapi/apikey")
            .headers(self.build_headers(true))
            .json(&post_body)
            .send()
            .await?;

        if !post_resp.status().is_success() {
            return Err(anyhow::anyhow!("POST apikey failed: {}", post_resp.status()));
        }

        let json_post: IFlowApiKeyResponse = post_resp.json().await?;
        if !json_post.success {
            return Err(anyhow::anyhow!("POST apikey returned false"));
        }

        let post_data = json_post.data.ok_or_else(|| anyhow::anyhow!("Missing POST response payload"))?;

        let token_info = TokenStorage {
            access_token: post_data.api_key,
            refresh_token: None,
            expires_at: Some(chrono::Utc::now() + chrono::Duration::days(7)),
            email: None,
            provider: "IFlow".to_string(),
            extra: std::collections::HashMap::new(),
        };

        if let Some(path) = &self.cache_path {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(path, serde_json::to_string_pretty(&token_info)?);
        }

        self.token = Some(token_info);
        Ok(())
    }

    fn inject(&self, req: RequestBuilder) -> anyhow::Result<RequestBuilder> {
        if let Some(t) = &self.token {
            Ok(req.bearer_auth(&t.access_token))
        } else {
            Err(anyhow::anyhow!("IFlowAuth not initialized"))
        }
    }

    fn auth_type(&self) -> AuthType {
        AuthType::Cookie
    }
}
