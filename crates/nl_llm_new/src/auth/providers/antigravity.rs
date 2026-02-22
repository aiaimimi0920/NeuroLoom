use crate::auth::AuthError;
use serde::{Deserialize, Serialize};
use url::Url;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Antigravity OAuth 配置
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

#[derive(Debug, Clone)]
pub struct AntigravityOAuthConfig {
    pub client_id: &'static str,
    pub client_secret: &'static str,
    pub redirect_port: u16,
    pub auth_url: &'static str,
    pub token_url: &'static str,
    pub scopes: &'static [&'static str],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntigravityToken {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub project_id: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OAuthTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: i64,
}

pub struct AntigravityOAuth {
    path: Option<PathBuf>,
    pub token: Option<AntigravityToken>,
}

impl AntigravityOAuth {
    pub fn new() -> Self {
        Self { path: None, token: None }
    }

    pub fn from_file(path: &Path) -> Result<Self, AuthError> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            let token: AntigravityToken = serde_json::from_str(&content)?;
            Ok(Self { path: Some(path.to_path_buf()), token: Some(token) })
        } else {
            Ok(Self { path: Some(path.to_path_buf()), token: None })
        }
    }

    pub async fn ensure_authenticated(&mut self) -> Result<(), AuthError> {
        if let Some(ref mut token) = self.token {
            if token.expires_at <= chrono::Utc::now() + chrono::Duration::seconds(300) {
                *token = Self::refresh_token_data(token).await?;
                if let Some(ref path) = self.path {
                    let _ = Self::save_token_to_path(token, path);
                }
            }
            Ok(())
        } else {
            let token = Self::login().await?;
            self.token = Some(token.clone());
            if let Some(ref path) = self.path {
                let _ = Self::save_token_to_path(&token, path);
            }
            Ok(())
        }
    }

    pub async fn login() -> Result<AntigravityToken, AuthError> {
        let state = format!("{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos());
        let redirect_uri = format!("http://127.0.0.1:{}/oauth-callback", ANTIGRAVITY_OAUTH_CONFIG.redirect_port);
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
        ).unwrap();

        println!("\n=== Antigravity OAuth Login ===");
        println!("请在浏览器中打开以下链接完成登录:\n{}\n", auth_url);
        
        // 尝试自动打开浏览器（仅在 Windows 等支持此用法的平台上）
        #[cfg(target_os = "windows")]
        let _ = std::process::Command::new("cmd").args(["/C", "start", auth_url.as_str()]).spawn();

        let (tx, rx) = std::sync::mpsc::channel();
        let server = tiny_http::Server::http(format!("127.0.0.1:{}", ANTIGRAVITY_OAUTH_CONFIG.redirect_port))
            .map_err(|e| AuthError::OAuthFailed(format!("启动回调服务器失败: {}", e)))?;

        std::thread::spawn(move || {
            for request in server.incoming_requests() {
                let url = request.url().to_string();
                if url.contains("code=") {
                    let code = url.split("code=").nth(1).unwrap_or("").split('&').next().unwrap_or("").to_string();
                    let html = "<h1>Login successful</h1><p>You can close this window.</p>";
                    let response = tiny_http::Response::from_string(html).with_header("Content-Type: text/html".parse::<tiny_http::Header>().unwrap());
                    let _ = request.respond(response);
                    let _ = tx.send(Ok(code));
                    break;
                } else if url.contains("error=") {
                    let err = url.split("error=").nth(1).unwrap_or("unknown").split('&').next().unwrap_or("unknown").to_string();
                    let _ = request.respond(tiny_http::Response::from_string("<h1>Login failed</h1><p>Check console output.</p>"));
                    let _ = tx.send(Err(err));
                    break;
                }
            }
        });

        println!("正在等待回调 (端口: {})...", ANTIGRAVITY_OAUTH_CONFIG.redirect_port);
        let code: String = match rx.recv() {
            Ok(Ok(code)) => code,
            Ok(Err(e)) => return Err(AuthError::OAuthFailed(format!("OAuth 错误: {}", e))),
            Err(_) => return Err(AuthError::OAuthFailed("未收到认证授权码".to_string())),
        };

        let token_resp = Self::exchange_code(&code, &redirect_uri).await?;
        let project_id = Self::fetch_project_id(&token_resp.access_token).await.ok();

        Ok(AntigravityToken {
            access_token: token_resp.access_token,
            refresh_token,
            expires_at,
            project_id,
            email: None,
        })
    }

    async fn exchange_code(code: &str, redirect_uri: &str) -> Result<OAuthTokenResponse, AuthError> {
        let params = [
            ("code", code),
            ("client_id", ANTIGRAVITY_OAUTH_CONFIG.client_id),
            ("client_secret", ANTIGRAVITY_OAUTH_CONFIG.client_secret),
            ("redirect_uri", redirect_uri),
            ("grant_type", "authorization_code"),
        ];

        let client = reqwest::Client::new();
        let res = client.post(ANTIGRAVITY_OAUTH_CONFIG.token_url).form(&params).send().await
            .map_err(|e| AuthError::OAuthFailed(format!("Token exchange request failed: {}", e)))?;

        if !res.status().is_success() {
            let body = res.text().await.unwrap_or_default();
            return Err(AuthError::OAuthFailed(format!("Token exchange failed: {}", body)));
        }

        res.json::<OAuthTokenResponse>().await.map_err(|e| AuthError::Http(e.to_string()))
    }

    async fn refresh_token_data(existing_token: &AntigravityToken) -> Result<AntigravityToken, AuthError> {
        let refresh_token_str = existing_token.refresh_token.clone();
        let params = [
            ("client_id", ANTIGRAVITY_OAUTH_CONFIG.client_id),
            ("client_secret", ANTIGRAVITY_OAUTH_CONFIG.client_secret),
            ("refresh_token", &refresh_token_str),
            ("grant_type", "refresh_token"),
        ];

        let client = reqwest::Client::new();
        let res = client.post(ANTIGRAVITY_OAUTH_CONFIG.token_url).form(&params).send().await
            .map_err(|e| AuthError::RefreshFailed(format!("Refresh request failed: {}", e)))?;

        if !res.status().is_success() {
            let body = res.text().await.unwrap_or_default();
            return Err(AuthError::RefreshFailed(format!("Token refresh failed: {}", body)));
        }

        let token_resp = res.json::<OAuthTokenResponse>().await.map_err(|e| AuthError::Http(e.to_string()))?;
        Ok(AntigravityToken {
            access_token: token_resp.access_token,
            refresh_token: token_resp.refresh_token.unwrap_or_else(|| refresh_token_str),
            expires_at: chrono::Utc::now() + chrono::Duration::seconds(token_resp.expires_in),
            project_id: existing_token.project_id.clone(),
            email: existing_token.email.clone(),
        })
    }

    fn save_token_to_path(token: &AntigravityToken, path: &Path) -> Result<(), std::io::Error> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(token)?)
    }

    pub fn access_token(&self) -> Option<&str> {
        self.token.as_ref().map(|t| t.access_token.as_str())
    }
}
