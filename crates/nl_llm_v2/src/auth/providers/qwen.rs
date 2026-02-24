use std::path::{Path, PathBuf};
use std::time::Duration;
use async_trait::async_trait;
use reqwest::{Client, RequestBuilder};
use serde::Deserialize;

use crate::auth::traits::Authenticator;
use crate::auth::types::{TokenStatus, TokenStorage};
use crate::site::context::AuthType;

// ── Qwen OAuth 常量 ──────────────────────────────────────────────────────────

const QWEN_DEVICE_CODE_ENDPOINT: &str = "https://chat.qwen.ai/api/v1/oauth2/device/code";
const QWEN_TOKEN_ENDPOINT: &str = "https://chat.qwen.ai/api/v1/oauth2/token";
const QWEN_CLIENT_ID: &str = "f0304373b74a44d2b584a3fb70ca9e56";
const QWEN_SCOPE: &str = "openid profile email model.completion";
const QWEN_DEVICE_CODE_GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";
const QWEN_USER_AGENT: &str = "QwenCode/0.10.3 (windows; x86_64)";

// ── 响应类型 ─────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct DeviceFlowResponse {
    device_code: String,
    user_code: String,
    #[allow(dead_code)]
    verification_uri: Option<String>,
    verification_uri_complete: String,
    #[allow(dead_code)]
    expires_in: Option<i64>,
    interval: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct QwenTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    #[allow(dead_code)]
    token_type: String,
    resource_url: Option<String>,
    expires_in: i64,
}

// ── QwenOAuth 认证器 ─────────────────────────────────────────────────────────

/// Qwen OAuth 认证器 (Device Code + PKCE)
///
/// 授权流程：
/// 1. POST device/code → 获取 device_code + verification_uri
/// 2. 打开浏览器让用户授权
/// 3. 轮询 token endpoint 直到用户完成授权
/// 4. 获取 access_token + refresh_token
///
/// API 调用时注入：
/// - `Authorization: Bearer {access_token}`
/// - `X-Dashscope-Authtype: qwen-oauth`
/// - `User-Agent: QwenCode/0.10.3`
pub struct QwenOAuth {
    token: Option<TokenStorage>,
    cache_path: Option<PathBuf>,
    http: Client,
}

impl QwenOAuth {
    pub fn new() -> Self {
        Self {
            token: None,
            cache_path: None,
            http: Client::builder().timeout(Duration::from_secs(30)).build().unwrap(),
        }
    }

    /// 设置 Token 缓存文件路径
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

    // ── PKCE ─────────────────────────────────────────────────────────────

    fn generate_code_verifier() -> String {
        use rand::RngCore;
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        base64_url_encode(&bytes)
    }

    fn generate_code_challenge(verifier: &str) -> String {
        use sha2::{Sha256, Digest};
        let hash = Sha256::digest(verifier.as_bytes());
        base64_url_encode(&hash)
    }

    // ── Device Code 授权流程 ──────────────────────────────────────────────

    async fn do_login(&self) -> anyhow::Result<TokenStorage> {
        let code_verifier = Self::generate_code_verifier();
        let code_challenge = Self::generate_code_challenge(&code_verifier);

        // Step 1: 请求 Device Code
        let params = [
            ("client_id", QWEN_CLIENT_ID),
            ("scope", QWEN_SCOPE),
            ("code_challenge", &code_challenge),
            ("code_challenge_method", "S256"),
        ];

        let res = self.http
            .post(QWEN_DEVICE_CODE_ENDPOINT)
            .form(&params)
            .header("Accept", "application/json")
            .send()
            .await?;

        if !res.status().is_success() {
            let body = res.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Device Code 请求失败: {}", body));
        }

        let device_flow: DeviceFlowResponse = res.json().await?;

        // Step 2: 提示用户授权
        println!("=== Qwen OAuth Login ===");
        println!("User Code: {}", device_flow.user_code);
        println!("请在浏览器中打开以下链接完成授权:\n{}\n", device_flow.verification_uri_complete);

        #[cfg(target_os = "windows")]
        let _ = std::process::Command::new("powershell")
            .args(["-Command", &format!("Start-Process '{}'", device_flow.verification_uri_complete)])
            .spawn();

        #[cfg(target_os = "macos")]
        let _ = std::process::Command::new("open")
            .arg(&device_flow.verification_uri_complete)
            .spawn();

        #[cfg(target_os = "linux")]
        let _ = std::process::Command::new("xdg-open")
            .arg(&device_flow.verification_uri_complete)
            .spawn();

        // Step 3: 轮询 Token
        println!("等待 Qwen 授权...");
        let poll_interval = std::cmp::max(device_flow.interval.unwrap_or(5), 5) as u64;
        let max_attempts = 60u32; // 5 分钟最大等待

        for attempt in 0..max_attempts {
            tokio::time::sleep(Duration::from_secs(poll_interval)).await;

            let token_params = [
                ("grant_type", QWEN_DEVICE_CODE_GRANT_TYPE),
                ("client_id", QWEN_CLIENT_ID),
                ("device_code", &device_flow.device_code),
                ("code_verifier", &code_verifier),
            ];

            let res = self.http
                .post(QWEN_TOKEN_ENDPOINT)
                .form(&token_params)
                .header("Accept", "application/json")
                .send()
                .await;

            let res = match res {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("轮询 {}/{} 失败: {}", attempt + 1, max_attempts, e);
                    continue;
                }
            };

            if res.status().is_success() {
                let token_resp: QwenTokenResponse = res.json().await?;
                let expires_at = chrono::Utc::now() + chrono::Duration::seconds(token_resp.expires_in);

                let mut extra = std::collections::HashMap::new();
                if let Some(ref url) = token_resp.resource_url {
                    extra.insert("resource_url".to_string(), serde_json::Value::String(url.clone()));
                }

                println!("Qwen 授权成功！");

                return Ok(TokenStorage {
                    access_token: token_resp.access_token,
                    refresh_token: token_resp.refresh_token,
                    expires_at: Some(expires_at),
                    email: None,
                    provider: "qwen".to_string(),
                    extra,
                });
            }

            // 解析错误响应
            let body = res.bytes().await.unwrap_or_default();
            if let Ok(error_data) = serde_json::from_slice::<serde_json::Value>(&body) {
                let error_type = error_data.get("error").and_then(|e| e.as_str()).unwrap_or("");
                match error_type {
                    "authorization_pending" => {
                        print!("\r轮询 {}/{}...", attempt + 1, max_attempts);
                        use std::io::Write;
                        let _ = std::io::stdout().flush();
                        continue;
                    }
                    "slow_down" => {
                        eprintln!("\n服务器要求降速，等待中...");
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                    "expired_token" => {
                        return Err(anyhow::anyhow!("Device Code 已过期，请重新登录"));
                    }
                    "access_denied" => {
                        return Err(anyhow::anyhow!("用户拒绝了授权"));
                    }
                    _ => {
                        let desc = error_data.get("error_description")
                            .and_then(|d| d.as_str())
                            .unwrap_or("unknown");
                        return Err(anyhow::anyhow!("Token 轮询失败: {} - {}", error_type, desc));
                    }
                }
            }
        }

        Err(anyhow::anyhow!("授权超时，请重新登录"))
    }

    // ── Token 刷新 ───────────────────────────────────────────────────────

    async fn do_refresh(&self, refresh_token: &str) -> anyhow::Result<TokenStorage> {
        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", QWEN_CLIENT_ID),
        ];

        let res = self.http
            .post(QWEN_TOKEN_ENDPOINT)
            .form(&params)
            .header("Accept", "application/json")
            .send()
            .await?;

        if !res.status().is_success() {
            let body = res.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Token 刷新失败: {}", body));
        }

        let token_resp: QwenTokenResponse = res.json().await?;
        let expires_at = chrono::Utc::now() + chrono::Duration::seconds(token_resp.expires_in);

        let mut extra = std::collections::HashMap::new();
        if let Some(ref url) = token_resp.resource_url {
            extra.insert("resource_url".to_string(), serde_json::Value::String(url.clone()));
        }

        Ok(TokenStorage {
            access_token: token_resp.access_token,
            refresh_token: Some(token_resp.refresh_token.unwrap_or_else(|| refresh_token.to_string())),
            expires_at: Some(expires_at),
            email: None,
            provider: "qwen".to_string(),
            extra,
        })
    }

    /// 获取 resource_url（API Base URL）
    pub fn resource_url(&self) -> Option<String> {
        self.token.as_ref()
            .and_then(|t| t.extra.get("resource_url"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }
}

// ── Authenticator 实现 ───────────────────────────────────────────────────────

#[async_trait]
impl Authenticator for QwenOAuth {
    fn id(&self) -> &str {
        "qwen_oauth"
    }

    fn is_authenticated(&self) -> bool {
        self.token.is_some()
    }

    fn needs_refresh(&self) -> bool {
        self.token.as_ref().map_or(true, |t| {
            // 提前 10 分钟刷新
            matches!(t.status(600), TokenStatus::Expired | TokenStatus::ExpiringSoon)
        })
    }

    async fn refresh(&mut self) -> anyhow::Result<()> {
        let new_token = if let Some(t) = &self.token {
            if let Some(rt) = &t.refresh_token {
                match self.do_refresh(rt).await {
                    Ok(token) => token,
                    Err(_) => {
                        // 刷新失败时回退到重新登录
                        self.do_login().await?
                    }
                }
            } else {
                self.do_login().await?
            }
        } else {
            self.do_login().await?
        };

        // 持久化到文件
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
            Ok(req
                .bearer_auth(&t.access_token)
                .header("User-Agent", QWEN_USER_AGENT)
                .header("X-Dashscope-Useragent", QWEN_USER_AGENT)
                .header("X-Dashscope-Authtype", "qwen-oauth")
                .header("X-Dashscope-Cachecontrol", "enable")
            )
        } else {
            Err(anyhow::anyhow!("Qwen 未认证"))
        }
    }

    fn auth_type(&self) -> AuthType {
        AuthType::OAuth
    }

    fn get_extra<'a>(&'a self) -> Option<&'a std::collections::HashMap<String, serde_json::Value>> {
        self.token.as_ref().map(|t| &t.extra)
    }
}

impl Default for QwenOAuth {
    fn default() -> Self {
        Self::new()
    }
}

// ── 工具函数 ─────────────────────────────────────────────────────────────────

fn base64_url_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}
