//! Antigravity (Gemini Code Assist) OAuth 认证实现
//!
//! 使用 Google OAuth2 流程获取 Access Token，用于调用 Cloud Code PA API。

use crate::auth::{AuthError, TokenStorage, TokenStatus};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use url::Url;

/// Antigravity OAuth 配置常量
pub const ANTIGRAVITY_OAUTH_CONFIG: AntigravityOAuthConfig = AntigravityOAuthConfig {
    client_id: "1071006060591-tmhssin2h21lcre235vtolojh4g403ep.apps.googleusercontent.com",
    client_secret: "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",
    redirect_port: 51121,
    auth_url: "https://accounts.google.com/o/oauth2/v2/auth",
    token_url: "https://oauth2.googleapis.com/token",
    scopes: &[
        "https://www.googleapis.com/auth/cloud-platform",
        "https://www.googleapis.com/auth/userinfo.email",
        "https://www.googleapis.com/auth/userinfo.profile",
        "https://www.googleapis.com/auth/cclog",
        "https://www.googleapis.com/auth/experimentsandconfigs",
    ],
};

/// Provider 标识
const PROVIDER_NAME: &str = "antigravity";

/// Antigravity OAuth 配置
#[derive(Debug, Clone)]
pub struct AntigravityOAuthConfig {
    pub client_id: &'static str,
    pub client_secret: &'static str,
    pub redirect_port: u16,
    pub auth_url: &'static str,
    pub token_url: &'static str,
    pub scopes: &'static [&'static str],
}

/// OAuth Token 响应
#[derive(Debug, Deserialize)]
struct OAuthTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: i64,
}

/// Antigravity OAuth 认证客户端
pub struct AntigravityOAuth {
    /// Token 文件路径
    path: Option<PathBuf>,
    /// 当前 Token（使用通用的 TokenStorage）
    token: Option<TokenStorage>,
    /// 复用的 HTTP Client
    http: reqwest::Client,
}

impl AntigravityOAuth {
    /// 创建新的认证客户端
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client for AntigravityOAuth");

        Self {
            path: None,
            token: None,
            http,
        }
    }

    /// 从文件加载 Token
    pub fn from_file(path: &Path) -> Result<Self, AuthError> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client for AntigravityOAuth");

        if path.exists() {
            let mut token = crate::auth::storage::TokenStorageManager::load(path)?;

            // 修正 provider 字段（向后兼容旧格式）
            if token.provider.is_empty() {
                token.provider = PROVIDER_NAME.to_string();
            }

            Ok(Self {
                path: Some(path.to_path_buf()),
                token: Some(token),
                http,
            })
        } else {
            Ok(Self {
                path: Some(path.to_path_buf()),
                token: None,
                http,
            })
        }
    }

    /// 确保已认证（自动刷新或登录）
    pub async fn ensure_authenticated(&mut self) -> Result<(), AuthError> {
        if self.token.is_none() {
            // 需要登录
            let token = self.login().await?;
            self.token = Some(token.clone());
            if let Some(ref path) = self.path {
                let _ = crate::auth::storage::TokenStorageManager::save(&token, path);
            }
            return Ok(());
        }

        // 检查是否需要刷新
        if self.needs_refresh() {
            let old_token = self.token.clone().unwrap();
            let new_token = Self::refresh_token_data_static(&self.http, &old_token).await?;
            self.token = Some(new_token.clone());
            if let Some(ref path) = self.path {
                let _ = crate::auth::storage::TokenStorageManager::save(&new_token, path);
            }
        }

        // 确保 project_id 存在
        let needs_project_id = self.project_id().map_or(true, |p| p.starts_with("project-"));

        if needs_project_id {
            let access_token = self.token.as_ref().unwrap().access_token.clone();
            if let Ok(pid) = Self::fetch_project_id_static(&self.http, &access_token).await {
                if let Some(ref mut token) = self.token {
                    token.extra.insert("project_id".to_string(), serde_json::Value::String(pid));
                    if let Some(ref path) = self.path {
                        let _ = crate::auth::storage::TokenStorageManager::save(token, path);
                    }
                }
            }
        }

        Ok(())
    }

    /// 获取 Token 状态
    pub fn token_status(&self) -> TokenStatus {
        self.token.as_ref().map_or(TokenStatus::Expired, |t| {
            t.status(300)
        })
    }

    /// 检查是否需要刷新
    pub fn needs_refresh(&self) -> bool {
        self.token.as_ref().map_or(true, |t| t.needs_refresh(300))
    }

    /// 获取 Access Token
    pub fn access_token(&self) -> Option<&str> {
        self.token.as_ref().map(|t| t.access_token.as_str())
    }

    /// 获取 Project ID
    pub fn project_id(&self) -> Option<&str> {
        self.token.as_ref().and_then(|t| {
            t.extra.get("project_id").and_then(|v| v.as_str())
        })
    }

    /// 获取 Token 存储（只读）
    pub fn token_storage(&self) -> Option<&TokenStorage> {
        self.token.as_ref()
    }

    /// OAuth 登录流程
    async fn login(&mut self) -> Result<TokenStorage, AuthError> {
        let state = format!(
            "{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let redirect_uri = format!(
            "http://127.0.0.1:{}/oauth-callback",
            ANTIGRAVITY_OAUTH_CONFIG.redirect_port
        );
        let auth_url = Url::parse_with_params(
            ANTIGRAVITY_OAUTH_CONFIG.auth_url,
            &[
                ("access_type", "offline"),
                ("client_id", ANTIGRAVITY_OAUTH_CONFIG.client_id),
                ("prompt", "consent"),
                ("redirect_uri", &redirect_uri),
                ("response_type", "code"),
                ("scope", &ANTIGRAVITY_OAUTH_CONFIG.scopes.join(" ")),
                ("state", &state),
            ],
        )
        .unwrap();

        println!("\n=== Antigravity OAuth Login ===");
        println!("请在浏览器中打开以下链接完成登录:\n{}\n", auth_url);

        // 尝试自动打开浏览器
        #[cfg(target_os = "windows")]
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", auth_url.as_str()])
            .spawn();

        let (tx, rx) = std::sync::mpsc::channel();
        let server = tiny_http::Server::http(format!(
            "127.0.0.1:{}",
            ANTIGRAVITY_OAUTH_CONFIG.redirect_port
        ))
        .map_err(|e| AuthError::OAuthFailed(format!("启动回调服务器失败: {}", e)))?;

        std::thread::spawn(move || {
            for request in server.incoming_requests() {
                let url = request.url().to_string();
                if url.contains("code=") {
                    let code = url
                        .split("code=")
                        .nth(1)
                        .unwrap_or("")
                        .split('&')
                        .next()
                        .unwrap_or("")
                        .to_string();
                    let html = "<h1>Login successful</h1><p>You can close this window.</p>";
                    let response = tiny_http::Response::from_string(html)
                        .with_header("Content-Type: text/html".parse::<tiny_http::Header>().unwrap());
                    let _ = request.respond(response);
                    let _ = tx.send(Ok(code));
                    break;
                } else if url.contains("error=") {
                    let err = url
                        .split("error=")
                        .nth(1)
                        .unwrap_or("unknown")
                        .split('&')
                        .next()
                        .unwrap_or("unknown")
                        .to_string();
                    let _ = request.respond(tiny_http::Response::from_string(
                        "<h1>Login failed</h1><p>Check console output.</p>",
                    ));
                    let _ = tx.send(Err(err));
                    break;
                }
            }
        });

        println!(
            "正在等待回调 (端口: {})...",
            ANTIGRAVITY_OAUTH_CONFIG.redirect_port
        );
        let code: String = match rx.recv() {
            Ok(Ok(code)) => code,
            Ok(Err(e)) => {
                return Err(AuthError::OAuthFailed(format!("OAuth 错误: {}", e)))
            }
            Err(_) => return Err(AuthError::OAuthFailed("未收到认证授权码".to_string())),
        };

        let token_resp = Self::exchange_code_static(&self.http, &code, &redirect_uri).await?;
        let expires_at = chrono::Utc::now() + chrono::Duration::seconds(token_resp.expires_in);
        let refresh_token = token_resp.refresh_token.ok_or_else(|| {
            AuthError::OAuthFailed("No refresh_token returned".to_string())
        })?;
        let project_id = Self::fetch_project_id_static(&self.http, &token_resp.access_token).await.ok();

        // 构建 TokenStorage
        let mut storage = TokenStorage::new(token_resp.access_token, PROVIDER_NAME)
            .with_refresh_token(refresh_token)
            .with_expires_at(expires_at);

        if let Some(pid) = project_id {
            storage = storage.with_extra("project_id", serde_json::Value::String(pid));
        }

        Ok(storage)
    }

    /// 用授权码换取 Token（静态方法）
    async fn exchange_code_static(
        http: &reqwest::Client,
        code: &str,
        redirect_uri: &str,
    ) -> Result<OAuthTokenResponse, AuthError> {
        let params = [
            ("code", code),
            ("client_id", ANTIGRAVITY_OAUTH_CONFIG.client_id),
            ("client_secret", ANTIGRAVITY_OAUTH_CONFIG.client_secret),
            ("redirect_uri", redirect_uri),
            ("grant_type", "authorization_code"),
        ];

        let res = http
            .post(ANTIGRAVITY_OAUTH_CONFIG.token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| {
                AuthError::OAuthFailed(format!("Token exchange request failed: {}", e))
            })?;

        if !res.status().is_success() {
            let body = res.text().await.unwrap_or_default();
            return Err(AuthError::OAuthFailed(format!(
                "Token exchange failed: {}",
                body
            )));
        }

        res.json::<OAuthTokenResponse>()
            .await
            .map_err(|e| AuthError::Http(e.to_string()))
    }

    /// 刷新 Token（静态方法）
    async fn refresh_token_data_static(
        http: &reqwest::Client,
        existing_token: &TokenStorage,
    ) -> Result<TokenStorage, AuthError> {
        let refresh_token_str = existing_token.refresh_token.clone().ok_or_else(|| {
            AuthError::RefreshFailed("No refresh_token available".to_string())
        })?;

        let params = [
            ("client_id", ANTIGRAVITY_OAUTH_CONFIG.client_id),
            ("client_secret", ANTIGRAVITY_OAUTH_CONFIG.client_secret),
            ("refresh_token", &refresh_token_str),
            ("grant_type", "refresh_token"),
        ];

        let res = http
            .post(ANTIGRAVITY_OAUTH_CONFIG.token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| AuthError::RefreshFailed(format!("Refresh request failed: {}", e)))?;

        if !res.status().is_success() {
            let body = res.text().await.unwrap_or_default();
            return Err(AuthError::RefreshFailed(format!(
                "Token refresh failed: {}",
                body
            )));
        }

        let token_resp = res
            .json::<OAuthTokenResponse>()
            .await
            .map_err(|e| AuthError::Http(e.to_string()))?;

        // 构建新的 TokenStorage，保留 extra 字段
        let mut storage = TokenStorage::new(token_resp.access_token, PROVIDER_NAME)
            .with_refresh_token(token_resp.refresh_token.unwrap_or(refresh_token_str))
            .with_expires_at(chrono::Utc::now() + chrono::Duration::seconds(token_resp.expires_in))
            .with_email_optional(existing_token.email.clone());

        // 保留原有的 extra 字段
        storage.extra = existing_token.extra.clone();

        Ok(storage)
    }

    /// 获取 Project ID（静态方法）
    async fn fetch_project_id_static(http: &reqwest::Client, access_token: &str) -> Result<String, AuthError> {
        let url = "https://cloudcode-pa.googleapis.com/v1internal:loadCodeAssist";
        let body = serde_json::json!({
            "metadata": {
                "ideType": "ANTIGRAVITY",
                "platform": "PLATFORM_UNSPECIFIED",
                "pluginType": "GEMINI"
            }
        });

        let res = http
            .post(url)
            .headers(Self::build_api_headers(access_token))
            .json(&body)
            .send()
            .await
            .map_err(|e| AuthError::Http(format!("loadCodeAssist request failed: {}", e)))?;

        let status = res.status();
        let text = res.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(AuthError::Http(format!(
                "loadCodeAssist returned {}: {}",
                status,
                text.trim()
            )));
        }

        let json: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| AuthError::Json(e))?;

        // 尝试直接获取 project_id
        let mut project_id = String::new();
        if let Some(id) = json.get("cloudaicompanionProject").and_then(|v| v.as_str()) {
            project_id = id.trim().to_string();
        } else if let Some(obj) = json
            .get("cloudaicompanionProject")
            .and_then(|v| v.as_object())
        {
            if let Some(id) = obj.get("id").and_then(|v| v.as_str()) {
                project_id = id.trim().to_string();
            }
        }

        if !project_id.is_empty() {
            return Ok(project_id);
        }

        // 尝试 onboard
        let tier_id = json
            .get("allowedTiers")
            .and_then(|v| v.as_array())
            .and_then(|tiers| {
                tiers.iter().find_map(|t| {
                    if t.get("isDefault").and_then(|v| v.as_bool()).unwrap_or(false) {
                        t.get("id").and_then(|v| v.as_str()).map(|s| s.to_string())
                    } else {
                        None
                    }
                })
            })
            .unwrap_or_else(|| "legacy-tier".to_string());

        Self::onboard_user_static(http, access_token, &tier_id).await
    }

    /// 用户 Onboard（静态方法）
    async fn onboard_user_static(http: &reqwest::Client, access_token: &str, tier_id: &str) -> Result<String, AuthError> {
        let url = "https://cloudcode-pa.googleapis.com/v1internal:onboardUser";
        let body = serde_json::json!({
            "tierId": tier_id,
            "metadata": {
                "ideType": "ANTIGRAVITY",
                "platform": "PLATFORM_UNSPECIFIED",
                "pluginType": "GEMINI"
            }
        });

        for _ in 1..=5 {
            let res = http
                .post(url)
                .headers(Self::build_api_headers(access_token))
                .json(&body)
                .send()
                .await
                .map_err(|e| AuthError::Http(format!("onboardUser request failed: {}", e)))?;

            let status = res.status();
            let text = res.text().await.unwrap_or_default();

            if !status.is_success() {
                return Err(AuthError::Http(format!(
                    "onboardUser returned {}: {}",
                    status,
                    text.trim()
                )));
            }

            let json: serde_json::Value =
                serde_json::from_str(&text).map_err(|e| AuthError::Json(e))?;

            if json.get("done").and_then(|v| v.as_bool()).unwrap_or(false) {
                let pid = json.get("response").and_then(|r| {
                    r.get("cloudaicompanionProject").and_then(|p| match p {
                        serde_json::Value::String(s) => Some(s.trim().to_string()),
                        serde_json::Value::Object(o) => o
                            .get("id")
                            .and_then(|v| v.as_str())
                            .map(|s| s.trim().to_string()),
                        _ => None,
                    })
                });

                return pid.ok_or_else(|| {
                    AuthError::Http("onboardUser done but no project_id in response".to_string())
                });
            }

            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }

        Err(AuthError::Http(
            "onboardUser: max attempts reached without completion".to_string(),
        ))
    }

    /// 构建 API 请求头
    fn build_api_headers(access_token: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&format!("Bearer {}", access_token)).unwrap(),
        );
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        headers.insert(
            "User-Agent",
            HeaderValue::from_static("google-cloud-sdk gcloud/0.0.0.dev"),
        );
        headers.insert(
            "X-Goog-Api-Client",
            HeaderValue::from_static("gl-python/3.12.0"),
        );
        headers.insert(
            "Client-Metadata",
            HeaderValue::from_static(
                r#"{"ideType":"IDE_UNSPECIFIED","platform":"PLATFORM_UNSPECIFIED","pluginType":"GEMINI"}"#,
            ),
        );
        headers
    }
}

impl Default for AntigravityOAuth {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_needs_refresh_without_token() {
        let auth = AntigravityOAuth::new();
        assert!(auth.needs_refresh());
        assert_eq!(auth.token_status(), TokenStatus::Expired);
    }
}
