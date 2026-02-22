use crate::auth::AuthError;
use serde::{Deserialize, Serialize};
use url::Url;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Gemini CLI OAuth 配置
pub const GEMINI_CLI_OAUTH_CONFIG: GeminiCliOAuthConfig = GeminiCliOAuthConfig {
    client_id: "681255809395-oo8ft2oprdrnp9e3aqf6av3hmdib135j.apps.googleusercontent.com",
    client_secret: "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",
    redirect_port: 8085,
    auth_url: "https://accounts.google.com/o/oauth2/v2/auth",
    token_url: "https://oauth2.googleapis.com/token",
    scopes: &[
        "https://www.googleapis.com/auth/cloud-platform",
        "https://www.googleapis.com/auth/userinfo.email",
        "https://www.googleapis.com/auth/userinfo.profile",
    ],
};

#[derive(Debug, Clone)]
pub struct GeminiCliOAuthConfig {
    pub client_id: &'static str,
    pub client_secret: &'static str,
    pub redirect_port: u16,
    pub auth_url: &'static str,
    pub token_url: &'static str,
    pub scopes: &'static [&'static str],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiCliToken {
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

pub struct GeminiCliOAuth {
    path: Option<PathBuf>,
    pub token: Option<GeminiCliToken>,
}

impl GeminiCliOAuth {
    pub fn new() -> Self {
        Self { path: None, token: None }
    }

    pub fn from_file(path: &Path) -> Result<Self, AuthError> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            let token: GeminiCliToken = serde_json::from_str(&content)?;
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
            if token.project_id.is_none() || token.project_id.as_ref().map(|p| p.starts_with("project-")).unwrap_or(false) {
                if let Ok(pid) = Self::fetch_project_id(&token.access_token).await {
                    token.project_id = Some(pid);
                    if let Some(ref path) = self.path {
                        let _ = Self::save_token_to_path(token, path);
                    }
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

    pub async fn login() -> Result<GeminiCliToken, AuthError> {
        let state = format!("{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos());
        let redirect_uri = format!("http://127.0.0.1:{}/oauth2callback", GEMINI_CLI_OAUTH_CONFIG.redirect_port);
        let auth_url = Url::parse_with_params(
            GEMINI_CLI_OAUTH_CONFIG.auth_url,
            &[
                ("access_type", "offline"),
                ("client_id", GEMINI_CLI_OAUTH_CONFIG.client_id),
                ("prompt", "consent"),
                ("redirect_uri", &redirect_uri),
                ("response_type", "code"),
                ("scope", &GEMINI_CLI_OAUTH_CONFIG.scopes.join(" ")),
                ("state", &state),
            ],
        ).unwrap();

        println!("=== Gemini CLI OAuth Login ===");
        println!("Please open the following URL in your browser:\n{}\n", auth_url);

        let (tx, rx) = std::sync::mpsc::channel();
        let server = tiny_http::Server::http(format!("127.0.0.1:{}", GEMINI_CLI_OAUTH_CONFIG.redirect_port))
            .map_err(|e| AuthError::OAuthFailed(format!("Failed to start callback server: {}", e)))?;

        std::thread::spawn(move || {
            for request in server.incoming_requests() {
                let url = request.url().to_string();
                if url.contains("code=") {
                    let code = url.split("code=").nth(1).unwrap_or("").split('&').next().unwrap_or("").to_string();
                    let response = tiny_http::Response::from_string("<h1>Login successful</h1><p>You can close this window.</p>")
                        .with_header("Content-Type: text/html".parse::<tiny_http::Header>().unwrap());
                    let _ = request.respond(response);
                    let _ = tx.send(Ok(code));
                    break;
                } else if url.contains("error=") {
                    let err = url.split("error=").nth(1).unwrap_or("unknown").split('&').next().unwrap_or("unknown").to_string();
                    let _ = request.respond(tiny_http::Response::from_string("<h1>Login failed</h1><p>Please check the CLI output.</p>"));
                    let _ = tx.send(Err(err));
                    break;
                }
            }
        });

        println!("正在等待回调 (端口: {})...", GEMINI_CLI_OAUTH_CONFIG.redirect_port);
        let code: String = match rx.recv() {
            Ok(Ok(code)) => code,
            Ok(Err(e)) => return Err(AuthError::OAuthFailed(format!("OAuth 错误: {}", e))),
            Err(_) => return Err(AuthError::OAuthFailed("未收到认证授权码".to_string())),
        };

        let token_resp = Self::exchange_code(&code, &redirect_uri).await?;
        let expires_at = chrono::Utc::now() + chrono::Duration::seconds(token_resp.expires_in);
        let refresh_token = token_resp.refresh_token.ok_or_else(|| AuthError::OAuthFailed("No refresh_token returned".to_string()))?;
        let project_id = Self::fetch_project_id(&token_resp.access_token).await.ok();

        Ok(GeminiCliToken {
            access_token: token_resp.access_token,
            refresh_token,
            expires_at,
            project_id,
            email: None,
        })
    }

    pub fn access_token(&self) -> Option<&str> {
        self.token.as_ref().map(|t| t.access_token.as_str())
    }

    async fn exchange_code(code: &str, redirect_uri: &str) -> Result<OAuthTokenResponse, AuthError> {
        let params = [
            ("code", code),
            ("client_id", GEMINI_CLI_OAUTH_CONFIG.client_id),
            ("client_secret", GEMINI_CLI_OAUTH_CONFIG.client_secret),
            ("redirect_uri", redirect_uri),
            ("grant_type", "authorization_code"),
        ];

        let client = reqwest::Client::new();
        let res = client.post(GEMINI_CLI_OAUTH_CONFIG.token_url).form(&params).send().await
            .map_err(|e| AuthError::OAuthFailed(format!("Token exchange request failed: {}", e)))?;

        if !res.status().is_success() {
            let body = res.text().await.unwrap_or_default();
            return Err(AuthError::OAuthFailed(format!("Token exchange failed: {}", body)));
        }

        res.json::<OAuthTokenResponse>().await
            .map_err(|e| AuthError::OAuthFailed(format!("Parse token response failed: {}", e)))
    }

    async fn refresh_token_data(existing: &GeminiCliToken) -> Result<GeminiCliToken, AuthError> {
        let params = [
            ("client_id", GEMINI_CLI_OAUTH_CONFIG.client_id),
            ("client_secret", GEMINI_CLI_OAUTH_CONFIG.client_secret),
            ("refresh_token", existing.refresh_token.as_str()),
            ("grant_type", "refresh_token"),
        ];

        let client = reqwest::Client::new();
        let res = client.post(GEMINI_CLI_OAUTH_CONFIG.token_url).form(&params).send().await
            .map_err(|e| AuthError::RefreshFailed(format!("Refresh request failed: {}", e)))?;

        if !res.status().is_success() {
            let body = res.text().await.unwrap_or_default();
            return Err(AuthError::RefreshFailed(format!("Token refresh failed: {}", body)));
        }

        let token_resp = res.json::<OAuthTokenResponse>().await
            .map_err(|e| AuthError::RefreshFailed(format!("Parse refresh response failed: {}", e)))?;

        Ok(GeminiCliToken {
            access_token: token_resp.access_token,
            refresh_token: token_resp.refresh_token.unwrap_or_else(|| existing.refresh_token.clone()),
            expires_at: chrono::Utc::now() + chrono::Duration::seconds(token_resp.expires_in),
            project_id: existing.project_id.clone(),
            email: existing.email.clone(),
        })
    }

    async fn fetch_project_id(access_token: &str) -> Result<String, AuthError> {
        let url = "https://cloudcode-pa.googleapis.com/v1internal:loadCodeAssist";
        let body = serde_json::json!({
            "metadata": {
                "ideType": "IDE_UNSPECIFIED",
                "platform": "PLATFORM_UNSPECIFIED",
                "pluginType": "GEMINI"
            }
        });

        let client = reqwest::Client::new();
        let res = client.post(url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .header("User-Agent", "google-api-nodejs-client/9.15.1")
            .header("X-Goog-Api-Client", "gl-node/22.17.0")
            .header("Client-Metadata", "ideType=IDE_UNSPECIFIED,platform=PLATFORM_UNSPECIFIED,pluginType=GEMINI")
            .json(&body)
            .send().await
            .map_err(|e| AuthError::OAuthFailed(format!("loadCodeAssist request failed: {}", e)))?;

        let status = res.status();
        let text = res.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(AuthError::OAuthFailed(format!("loadCodeAssist returned {}: {}", status, text.trim())));
        }

        let json: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();

        let mut project_id = String::new();
        if let Some(id) = json.get("cloudaicompanionProject").and_then(|v| v.as_str()) {
            project_id = id.trim().to_string();
        } else if let Some(obj) = json.get("cloudaicompanionProject").and_then(|v| v.as_object()) {
            if let Some(id) = obj.get("id").and_then(|v| v.as_str()) {
                project_id = id.trim().to_string();
            }
        }

        if !project_id.is_empty() {
            return Ok(project_id);
        }

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

        Self::onboard_user(access_token, &tier_id).await
    }

    async fn onboard_user(access_token: &str, tier_id: &str) -> Result<String, AuthError> {
        let url = "https://cloudcode-pa.googleapis.com/v1internal:onboardUser";
        let body = serde_json::json!({
            "tierId": tier_id,
            "metadata": {
                "ideType": "IDE_UNSPECIFIED",
                "platform": "PLATFORM_UNSPECIFIED",
                "pluginType": "GEMINI"
            }
        });

        let client = reqwest::Client::new();
        for _ in 1..=5 {
            let res = client.post(url)
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .header("User-Agent", "google-api-nodejs-client/9.15.1")
                .header("X-Goog-Api-Client", "gl-node/22.17.0")
                .header("Client-Metadata", "ideType=IDE_UNSPECIFIED,platform=PLATFORM_UNSPECIFIED,pluginType=GEMINI")
                .json(&body)
                .send().await
                .map_err(|e| AuthError::OAuthFailed(format!("onboardUser request failed: {}", e)))?;

            let status = res.status();
            let text = res.text().await.unwrap_or_default();

            if !status.is_success() {
                return Err(AuthError::OAuthFailed(format!("onboardUser returned {}: {}", status, text.trim())));
            }

            let json: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();

            if json.get("done").and_then(|v| v.as_bool()).unwrap_or(false) {
                let pid = json.get("response")
                    .and_then(|r| r.get("cloudaicompanionProject"))
                    .and_then(|p| match p {
                        serde_json::Value::String(s) => Some(s.trim().to_string()),
                        serde_json::Value::Object(o) => o.get("id").and_then(|v| v.as_str()).map(|s| s.trim().to_string()),
                        _ => None,
                    });

                return pid.ok_or_else(|| AuthError::OAuthFailed("onboardUser done but no project_id in response".to_string()));
            }

            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }

        Err(AuthError::OAuthFailed("onboardUser: max attempts reached without completion".to_string()))
    }

    fn save_token_to_path(token: &GeminiCliToken, path: &Path) -> Result<(), std::io::Error> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(token)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
