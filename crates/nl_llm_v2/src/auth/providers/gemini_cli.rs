use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use async_trait::async_trait;
use reqwest::{Client, RequestBuilder};
use serde::Deserialize;
use url::Url;

use crate::auth::traits::Authenticator;
use crate::auth::types::{TokenStatus, TokenStorage};
use crate::site::context::AuthType;

pub const GEMINI_CLI_OAUTH_CONFIG: GeminiCliOAuthConfig = GeminiCliOAuthConfig {
    client_id: "681255809395-oo8ft2oprdrnp9e3aqf6av3hmdib135j.apps.googleusercontent.com",
    client_secret: "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",
    redirect_port: 8085,
    auth_url: "https://accounts.google.com/o/oauth2/v2/auth",
    token_url: "https://oauth2.googleapis.com/token",
    scopes: &[
        "https://www.googleapis.com/auth/cloud-platform",
    ],
};

pub struct GeminiCliOAuthConfig {
    pub client_id: &'static str,
    pub client_secret: &'static str,
    pub redirect_port: u16,
    pub auth_url: &'static str,
    pub token_url: &'static str,
    pub scopes: &'static [&'static str],
}

#[derive(Debug, Deserialize)]
struct OAuthTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: i64,
}

pub struct GeminiCliOAuth {
    token: Option<TokenStorage>,
    cache_path: Option<PathBuf>,
    http: Client,
}

impl GeminiCliOAuth {
    pub fn new() -> Self {
        Self {
            token: None,
            cache_path: None,
            http: Client::builder().timeout(Duration::from_secs(30)).build().unwrap(),
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

    async fn do_login(&self) -> anyhow::Result<TokenStorage> {
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

        #[cfg(target_os = "windows")]
        let _ = std::process::Command::new("powershell")
            .args(["-Command", &format!("Start-Process '{}'", auth_url)])
            .spawn();

        let (tx, rx) = std::sync::mpsc::channel();
        let server = tiny_http::Server::http(format!("127.0.0.1:{}", GEMINI_CLI_OAUTH_CONFIG.redirect_port))
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

        println!("正在等待回调 (端口: {})...", GEMINI_CLI_OAUTH_CONFIG.redirect_port);
        let code: String = match rx.recv() {
            Ok(Ok(code)) => code,
            Ok(Err(e)) => return Err(anyhow::anyhow!("OAuth Error: {}", e)),
            Err(_) => return Err(anyhow::anyhow!("Did not receive auth code")),
        };

        let params = [
            ("code", code.as_str()),
            ("client_id", GEMINI_CLI_OAUTH_CONFIG.client_id),
            ("client_secret", GEMINI_CLI_OAUTH_CONFIG.client_secret),
            ("redirect_uri", &redirect_uri),
            ("grant_type", "authorization_code"),
        ];

        let res = self.http.post(GEMINI_CLI_OAUTH_CONFIG.token_url).form(&params).send().await?;
        if !res.status().is_success() {
            let body = res.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Token exchange failed: {}", body));
        }

        let token_resp: OAuthTokenResponse = res.json().await?;
        let expires_at = chrono::Utc::now() + chrono::Duration::seconds(token_resp.expires_in);

        let extra = std::collections::HashMap::new();

        Ok(TokenStorage {
            access_token: token_resp.access_token,
            refresh_token: token_resp.refresh_token,
            expires_at: Some(expires_at),
            email: None,
            provider: "GeminiCLI".to_string(),
            extra,
        })
    }

    async fn do_refresh(&self, refresh_token: &str) -> anyhow::Result<TokenStorage> {
        let params = [
            ("client_id", GEMINI_CLI_OAUTH_CONFIG.client_id),
            ("client_secret", GEMINI_CLI_OAUTH_CONFIG.client_secret),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ];

        let res = self.http.post(GEMINI_CLI_OAUTH_CONFIG.token_url).form(&params).send().await?;
        if !res.status().is_success() {
            let body = res.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Token refresh failed: {}", body));
        }

        let token_resp: OAuthTokenResponse = res.json().await?;
        let expires_at = chrono::Utc::now() + chrono::Duration::seconds(token_resp.expires_in);

        let extra = std::collections::HashMap::new();

        Ok(TokenStorage {
            access_token: token_resp.access_token,
            refresh_token: Some(token_resp.refresh_token.unwrap_or_else(|| refresh_token.to_string())),
            expires_at: Some(expires_at),
            email: None,
            provider: "GeminiCLI".to_string(),
            extra,
        })
    }
}

#[async_trait]
impl Authenticator for GeminiCliOAuth {
    fn id(&self) -> &str {
        "gemini_cli_oauth"
    }

    fn is_authenticated(&self) -> bool {
        self.token.is_some()
    }

    fn needs_refresh(&self) -> bool {
        self.token.as_ref().map_or(true, |t| {
            matches!(t.status(300), TokenStatus::Expired | TokenStatus::ExpiringSoon)
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
            // Note: cloudCode API needs specific headers which usually might go into a Hook or Site.
            // But we inject the Bearer here as Standard auth responsibility.
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
