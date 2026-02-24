use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use async_trait::async_trait;
use reqwest::{Client, RequestBuilder};
use serde::Deserialize;
use url::Url;

use crate::auth::traits::Authenticator;
use crate::auth::types::{TokenStatus, TokenStorage};
use crate::site::context::AuthType;

/// [保留] 默认 OAuth 配置常量
/// 原因：方便直接使用，无需每次构造
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

/// [优化] OAuth 配置，支持静态和动态两种模式
/// 原因：允许用户自定义 OAuth 配置（如使用自己的 OAuth 应用）
pub struct AntigravityOAuthConfig {
    pub client_id: &'static str,
    pub client_secret: &'static str,
    pub redirect_port: u16,
    pub auth_url: &'static str,
    pub token_url: &'static str,
    pub scopes: &'static [&'static str],
}

/// [新增] 动态 OAuth 配置（Owned 版本）
/// 原因：支持运行时构建的自定义配置
#[derive(Clone)]
pub struct DynamicOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_port: u16,
    pub auth_url: String,
    pub token_url: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct OAuthTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: i64,
}

/// [优化] OAuth 认证器，支持自定义配置
pub struct AntigravityOAuth {
    token: Option<TokenStorage>,
    cache_path: Option<PathBuf>,
    http: Client,
    /// [新增] 动态配置（可选）
    config: Option<DynamicOAuthConfig>,
    /// [新增] 是否启用详细日志
    verbose: bool,
}

impl AntigravityOAuth {
    pub fn new() -> Self {
        Self {
            token: None,
            cache_path: None,
            http: Client::builder().timeout(Duration::from_secs(30)).build().unwrap(),
            config: None,
            verbose: false,
        }
    }

    /// [新增] 设置缓存文件路径
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

    /// [新增] 使用自定义 OAuth 配置
    /// 原因：支持用户自己的 OAuth 应用
    pub fn with_config(mut self, config: DynamicOAuthConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// [新增] 启用详细日志
    /// 原因：调试时可看到更多错误信息
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// [辅助] 获取配置值的辅助方法
    fn client_id(&self) -> &str {
        self.config.as_ref()
            .map(|c| c.client_id.as_str())
            .unwrap_or(ANTIGRAVITY_OAUTH_CONFIG.client_id)
    }

    fn client_secret(&self) -> &str {
        self.config.as_ref()
            .map(|c| c.client_secret.as_str())
            .unwrap_or(ANTIGRAVITY_OAUTH_CONFIG.client_secret)
    }

    fn redirect_port(&self) -> u16 {
        self.config.as_ref()
            .map(|c| c.redirect_port)
            .unwrap_or(ANTIGRAVITY_OAUTH_CONFIG.redirect_port)
    }

    fn auth_url(&self) -> &str {
        self.config.as_ref()
            .map(|c| c.auth_url.as_str())
            .unwrap_or(ANTIGRAVITY_OAUTH_CONFIG.auth_url)
    }

    fn token_url(&self) -> &str {
        self.config.as_ref()
            .map(|c| c.token_url.as_str())
            .unwrap_or(ANTIGRAVITY_OAUTH_CONFIG.token_url)
    }

    fn scopes(&self) -> String {
        self.config.as_ref()
            .map(|c| c.scopes.join(" "))
            .unwrap_or_else(|| ANTIGRAVITY_OAUTH_CONFIG.scopes.join(" "))
    }

    fn log(&self, msg: &str) {
        if self.verbose {
            eprintln!("[AntigravityOAuth] {}", msg);
        }
    }

    /// [辅助] 获取 project_id 并插入 extra map，消除 do_login/do_refresh 重复代码
    async fn resolve_project_id_extra(&self, access_token: &str, context: &str) -> std::collections::HashMap<String, serde_json::Value> {
        let mut extra = std::collections::HashMap::new();
        match self.fetch_project_id(access_token).await {
            Ok(Some(pid)) => {
                extra.insert("project_id".to_string(), serde_json::Value::String(pid));
            }
            Ok(None) => {
                self.log(&format!("fetch_project_id returned None during {}", context));
            }
            Err(e) => {
                self.log(&format!("fetch_project_id error during {}: {}", context, e));
            }
        }
        extra
    }

    async fn do_login(&self) -> anyhow::Result<TokenStorage> {
        let state = format!("{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos());
        let redirect_port = self.redirect_port();
        let redirect_uri = format!("http://127.0.0.1:{}/oauth-callback", redirect_port);
        let auth_url = Url::parse_with_params(
            self.auth_url(),
            &[
                ("access_type", "offline"),
                ("client_id", self.client_id()),
                ("prompt", "consent"),
                ("redirect_uri", &redirect_uri),
                ("response_type", "code"),
                ("scope", &self.scopes()),
                ("state", &state),
            ],
        ).unwrap();

        println!("=== Antigravity OAuth Login ===");
        println!("Please open the following URL in your browser:\n{}\n", auth_url);

        #[cfg(target_os = "windows")]
        let _ = std::process::Command::new("powershell")
            .args(["-Command", &format!("Start-Process '{}'", auth_url)])
            .spawn();

        let (tx, rx) = std::sync::mpsc::channel();
        let server = tiny_http::Server::http(format!("127.0.0.1:{}", redirect_port))
            .map_err(|e| anyhow::anyhow!("Failed to start callback server: {}", e))?;

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

        println!("等待 Antigravity 登录回调 (端口: {})...", redirect_port);
        let code: String = match rx.recv() {
            Ok(Ok(code)) => code,
            Ok(Err(e)) => return Err(anyhow::anyhow!("OAuth Error: {}", e)),
            Err(_) => return Err(anyhow::anyhow!("Did not receive auth code")),
        };

        let params = [
            ("code", code.as_str()),
            ("client_id", self.client_id()),
            ("client_secret", self.client_secret()),
            ("redirect_uri", &redirect_uri),
            ("grant_type", "authorization_code"),
        ];

        let res = self.http.post(self.token_url()).form(&params).send().await?;
        if !res.status().is_success() {
            let body = res.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Token exchange failed: {}", body));
        }

        let token_resp: OAuthTokenResponse = res.json().await?;
        let expires_at = chrono::Utc::now() + chrono::Duration::seconds(token_resp.expires_in);

        let extra = self.resolve_project_id_extra(&token_resp.access_token, "login").await;

        Ok(TokenStorage {
            access_token: token_resp.access_token,
            refresh_token: token_resp.refresh_token,
            expires_at: Some(expires_at),
            email: None,
            provider: "Antigravity".to_string(),
            extra,
        })
    }

    async fn do_refresh(&self, refresh_token: &str) -> anyhow::Result<TokenStorage> {
        let params = [
            ("client_id", self.client_id()),
            ("client_secret", self.client_secret()),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ];

        let res = self.http.post(self.token_url()).form(&params).send().await?;
        if !res.status().is_success() {
            let body = res.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Token refresh failed: {}", body));
        }

        let token_resp: OAuthTokenResponse = res.json().await?;
        let expires_at = chrono::Utc::now() + chrono::Duration::seconds(token_resp.expires_in);

        let extra = self.resolve_project_id_extra(&token_resp.access_token, "refresh").await;

        Ok(TokenStorage {
            access_token: token_resp.access_token,
            refresh_token: Some(token_resp.refresh_token.unwrap_or_else(|| refresh_token.to_string())),
            expires_at: Some(expires_at),
            email: None,
            provider: "Antigravity".to_string(),
            extra,
        })
    }

    /// [优化] 获取 project_id，返回 Result 以提供更详细的错误信息
    /// 原因：失败时可以知道具体原因，便于调试
    async fn fetch_project_id(&self, access_token: &str) -> anyhow::Result<Option<String>> {
        let url = "https://cloudcode-pa.googleapis.com/v1internal:loadCodeAssist";
        let body = serde_json::json!({
            "metadata": {
                "ideType": "ANTIGRAVITY",
                "platform": "PLATFORM_UNSPECIFIED",
                "pluginType": "GEMINI"
            }
        });

        let res = self.http.post(url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .header("User-Agent", "google-api-nodejs-client/9.15.1")
            .header("X-Goog-Api-Client", "google-cloud-sdk vscode_cloudshelleditor/0.1")
            .header("Client-Metadata", r#"{"ideType":"IDE_UNSPECIFIED","platform":"PLATFORM_UNSPECIFIED","pluginType":"GEMINI"}"#)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("HTTP request failed: {}", e))?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            self.log(&format!("loadCodeAssist failed with status {}: {}", status, body));
            return Ok(None);
        }

        let json = res.json::<serde_json::Value>().await
            .map_err(|e| anyhow::anyhow!("Failed to parse JSON response: {}", e))?;

        let mut project_id = String::new();
        if let Some(id) = json.get("cloudaicompanionProject").and_then(|v| v.as_str()) {
            project_id = id.trim().to_string();
        } else if let Some(obj) = json.get("cloudaicompanionProject").and_then(|v| v.as_object()) {
            if let Some(id) = obj.get("id").and_then(|v| v.as_str()) {
                project_id = id.trim().to_string();
            }
        }

        if !project_id.is_empty() {
            self.log(&format!("Got project_id from loadCodeAssist: {}", project_id));
            return Ok(Some(project_id));
        }

        // 尝试通过 onboard_user 获取
        let tier_id = json.get("allowedTiers")
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

        self.log(&format!("Attempting onboard_user with tier_id: {}", tier_id));
        match self.onboard_user(access_token, &tier_id).await {
            Ok(Some(pid)) => Ok(Some(pid)),
            Ok(None) => {
                self.log("onboard_user returned None");
                Ok(None)
            }
            Err(e) => {
                self.log(&format!("onboard_user error: {}", e));
                Ok(None)
            }
        }
    }

    /// [优化] 用户入驻流程，返回 Result
    async fn onboard_user(&self, access_token: &str, tier_id: &str) -> anyhow::Result<Option<String>> {
        let url = "https://cloudcode-pa.googleapis.com/v1internal:onboardUser";
        let body = serde_json::json!({
            "tierId": tier_id,
            "metadata": {
                "ideType": "ANTIGRAVITY",
                "platform": "PLATFORM_UNSPECIFIED",
                "pluginType": "GEMINI"
            }
        });

        for attempt in 0..3 {
            let res = self.http.post(url)
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .header("User-Agent", "google-api-nodejs-client/9.15.1")
                .header("X-Goog-Api-Client", "google-cloud-sdk vscode_cloudshelleditor/0.1")
                .header("Client-Metadata", r#"{"ideType":"IDE_UNSPECIFIED","platform":"PLATFORM_UNSPECIFIED","pluginType":"GEMINI"}"#)
                .json(&body)
                .send()
                .await
                .map_err(|e| anyhow::anyhow!("HTTP request failed: {}", e))?;

            if res.status().is_success() {
                if let Ok(json) = res.json::<serde_json::Value>().await {
                    if let Some(id) = json.get("cloudaicompanionProject").and_then(|v| v.as_str()) {
                        return Ok(Some(id.trim().to_string()));
                    } else if let Some(obj) = json.get("cloudaicompanionProject").and_then(|v| v.as_object()) {
                        if let Some(id) = obj.get("id").and_then(|v| v.as_str()) {
                            return Ok(Some(id.trim().to_string()));
                        }
                    }
                }
            } else {
                self.log(&format!("onboardUser attempt {} failed with status {}", attempt + 1, res.status()));
            }
            tokio::time::sleep(Duration::from_millis(1000)).await;
        }

        Ok(None)
    }
}

#[async_trait]
impl Authenticator for AntigravityOAuth {
    fn id(&self) -> &str {
        "antigravity_oauth"
    }

    fn is_authenticated(&self) -> bool {
        self.token.is_some()
    }

    fn needs_refresh(&self) -> bool {
        self.token.as_ref().map_or(true, |t| {
            matches!(t.status(300), TokenStatus::Expired | TokenStatus::ExpiringSoon) || !t.extra.contains_key("project_id")
        })
    }

    async fn refresh(&mut self) -> anyhow::Result<()> {
        let new_token = if let Some(t) = &self.token {
            if let Some(rt) = &t.refresh_token {
                self.do_refresh(rt).await?
            } else {
                self.do_login().await?
            }
        } else {
            self.do_login().await?
        };

        if let Some(path) = &self.cache_path {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(path, serde_json::to_string_pretty(&new_token)?);
        }

        self.token = Some(new_token);
        Ok(())
    }

    fn inject(&self, req: RequestBuilder) -> anyhow::Result<RequestBuilder> {
        if let Some(t) = &self.token {
            Ok(req.bearer_auth(&t.access_token))
        } else {
            Err(anyhow::anyhow!("Not authenticated"))
        }
    }

    fn auth_type(&self) -> AuthType {
        AuthType::OAuth
    }

    fn get_extra<'a>(&'a self) -> Option<&'a std::collections::HashMap<String, serde_json::Value>> {
        self.token.as_ref().map(|t| &t.extra)
    }
}

impl Default for AntigravityOAuth {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for DynamicOAuthConfig {
    fn default() -> Self {
        Self {
            client_id: ANTIGRAVITY_OAUTH_CONFIG.client_id.to_string(),
            client_secret: ANTIGRAVITY_OAUTH_CONFIG.client_secret.to_string(),
            redirect_port: ANTIGRAVITY_OAUTH_CONFIG.redirect_port,
            auth_url: ANTIGRAVITY_OAUTH_CONFIG.auth_url.to_string(),
            token_url: ANTIGRAVITY_OAUTH_CONFIG.token_url.to_string(),
            scopes: ANTIGRAVITY_OAUTH_CONFIG.scopes.iter().map(|s| s.to_string()).collect(),
        }
    }
}
