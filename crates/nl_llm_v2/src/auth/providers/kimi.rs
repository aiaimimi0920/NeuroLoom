use async_trait::async_trait;
use reqwest::{Client, RequestBuilder};
use serde::Deserialize;
use std::path::PathBuf;
use std::collections::HashMap;

use crate::auth::traits::Authenticator;
use crate::site::context::AuthType;
use crate::auth::types::TokenStorage;

// === Constants ===
const KIMI_CLIENT_ID: &str = "17e5f671-d194-4dfb-9706-5516cb48c098";
#[allow(dead_code)]
const KIMI_OAUTH_HOST: &str = "https://auth.kimi.com";
const KIMI_DEVICE_CODE_URL: &str = "https://auth.kimi.com/api/oauth/device_authorization";
const KIMI_TOKEN_URL: &str = "https://auth.kimi.com/api/oauth/token";
const KIMI_USER_AGENT: &str = "KimiCLI/1.10.6";
const KIMI_PLATFORM: &str = "kimi_cli";
const KIMI_VERSION: &str = "1.10.6";

// === Response types ===

#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: Option<String>,
    #[allow(dead_code)]
    verification_uri: Option<String>,
    verification_uri_complete: String,
    #[allow(dead_code)]
    expires_in: Option<i64>,
    interval: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    refresh_token: Option<String>,
    #[allow(dead_code)]
    token_type: Option<String>,
    expires_in: Option<f64>,
    #[allow(dead_code)]
    scope: Option<String>,
    error: Option<String>,
    #[allow(dead_code)]
    error_description: Option<String>,
}

// === KimiOAuth ===

/// Kimi (Moonshot AI) OAuth 认证器
///
/// 使用 RFC 8628 Device Authorization Grant 流程。
/// - OAuth 端点: auth.kimi.com
/// - API 端点: api.kimi.com/coding
/// - Header: X-Msh-Platform / X-Msh-Version / X-Msh-Device-*
pub struct KimiOAuth {
    token: Option<TokenStorage>,
    cache_path: Option<PathBuf>,
    device_id: String,
    http: Client,
}

impl KimiOAuth {
    pub fn new(cache_path: impl AsRef<std::path::Path>) -> Self {
        let cache_path = cache_path.as_ref().to_path_buf();
        let device_id = uuid::Uuid::new_v4().to_string();

        let mut auth = Self {
            token: None,
            cache_path: Some(cache_path.clone()),
            device_id,
            http: Client::new(),
        };

        // 尝试从缓存加载 token
        if let Ok(content) = std::fs::read_to_string(&cache_path) {
            if let Ok(t) = serde_json::from_str::<TokenStorage>(&content) {
                // 恢复 device_id
                if let Some(did) = t.extra.get("device_id").and_then(|v| v.as_str()) {
                    auth.device_id = did.to_string();
                }
                auth.token = Some(t);
            }
        }

        auth
    }

    /// 保存 token 到缓存文件
    fn save_token(&self) -> anyhow::Result<()> {
        if let (Some(t), Some(path)) = (&self.token, &self.cache_path) {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let json = serde_json::to_string_pretty(t)?;
            std::fs::write(path, json)?;
        }
        Ok(())
    }

    /// 获取设备信息 Header
    fn device_model() -> String {
        format!("{} {}", std::env::consts::OS, std::env::consts::ARCH)
    }

    fn hostname() -> String {
        std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .unwrap_or_else(|_| "unknown".to_string())
    }

    /// Device Code 流程
    async fn do_login(&mut self) -> anyhow::Result<()> {
        println!("\n=== Kimi OAuth Login ===");

        // Step 1: 请求 device code
        let params = [("client_id", KIMI_CLIENT_ID)];

        let resp = self.http.post(KIMI_DEVICE_CODE_URL)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Accept", "application/json")
            .header("User-Agent", KIMI_USER_AGENT)
            .header("X-Msh-Platform", KIMI_PLATFORM)
            .header("X-Msh-Version", KIMI_VERSION)
            .header("X-Msh-Device-Name", Self::hostname())
            .header("X-Msh-Device-Model", Self::device_model())
            .header("X-Msh-Device-Id", &self.device_id)
            .form(&params)
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Kimi device code 请求失败: {}", body));
        }

        let device_flow: DeviceCodeResponse = resp.json().await?;

        // Step 2: 提示用户
        if let Some(code) = &device_flow.user_code {
            println!("User Code: {}", code);
        }
        println!("请在浏览器中打开以下链接完成授权:");
        println!("{}\n", device_flow.verification_uri_complete);

        // 尝试打开浏览器
        let _ = open::that(&device_flow.verification_uri_complete);

        // Step 3: 轮询 token
        println!("等待 Kimi 授权...");
        let poll_interval = std::cmp::max(device_flow.interval.unwrap_or(5), 5) as u64;
        let max_attempts = 180u32; // 15 分钟

        for attempt in 0..max_attempts {
            tokio::time::sleep(std::time::Duration::from_secs(poll_interval)).await;
            print!("轮询 {}/{}...", attempt + 1, max_attempts);

            let params = [
                ("client_id", KIMI_CLIENT_ID),
                ("device_code", device_flow.device_code.as_str()),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ];

            let resp = self.http.post(KIMI_TOKEN_URL)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .header("Accept", "application/json")
                .header("User-Agent", KIMI_USER_AGENT)
                .header("X-Msh-Platform", KIMI_PLATFORM)
                .header("X-Msh-Version", KIMI_VERSION)
                .header("X-Msh-Device-Name", Self::hostname())
                .header("X-Msh-Device-Model", Self::device_model())
                .header("X-Msh-Device-Id", &self.device_id)
                .form(&params)
                .send()
                .await?;

            let token_resp: TokenResponse = resp.json().await?;

            // 检查错误
            if let Some(err) = &token_resp.error {
                match err.as_str() {
                    "authorization_pending" | "slow_down" => continue,
                    "expired_token" => return Err(anyhow::anyhow!("Kimi device code 已过期")),
                    "access_denied" => return Err(anyhow::anyhow!("Kimi 用户拒绝授权")),
                    other => return Err(anyhow::anyhow!("Kimi OAuth 错误: {}", other)),
                }
            }

            if let Some(access_token) = token_resp.access_token {
                if access_token.is_empty() {
                    continue;
                }

                println!("Kimi 授权成功！");

                let expires_at = token_resp.expires_in.map(|e| {
                    chrono::Utc::now() + chrono::Duration::seconds(e as i64)
                });

                let mut extra = HashMap::new();
                extra.insert("device_id".to_string(), serde_json::Value::String(self.device_id.clone()));

                self.token = Some(TokenStorage {
                    access_token,
                    refresh_token: token_resp.refresh_token,
                    expires_at,
                    email: None,
                    provider: "kimi".to_string(),
                    extra,
                });

                self.save_token()?;
                return Ok(());
            }
        }

        Err(anyhow::anyhow!("Kimi 授权等待超时"))
    }

    /// 刷新 token
    async fn do_refresh(&mut self) -> anyhow::Result<()> {
        let refresh_token = self.token.as_ref()
            .and_then(|t| t.refresh_token.as_deref())
            .ok_or_else(|| anyhow::anyhow!("无 refresh_token"))?
            .to_string();

        let params = [
            ("client_id", KIMI_CLIENT_ID),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token.as_str()),
        ];

        let resp = self.http.post(KIMI_TOKEN_URL)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Accept", "application/json")
            .header("User-Agent", KIMI_USER_AGENT)
            .header("X-Msh-Platform", KIMI_PLATFORM)
            .header("X-Msh-Version", KIMI_VERSION)
            .header("X-Msh-Device-Name", Self::hostname())
            .header("X-Msh-Device-Model", Self::device_model())
            .header("X-Msh-Device-Id", &self.device_id)
            .form(&params)
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::UNAUTHORIZED || resp.status() == reqwest::StatusCode::FORBIDDEN {
            return Err(anyhow::anyhow!("Kimi refresh token 已失效，需要重新登录"));
        }

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Kimi token 刷新失败: {}", body));
        }

        let token_resp: TokenResponse = resp.json().await?;

        let access_token = token_resp.access_token
            .ok_or_else(|| anyhow::anyhow!("Kimi 刷新响应缺少 access_token"))?;

        let expires_at = token_resp.expires_in.map(|e| {
            chrono::Utc::now() + chrono::Duration::seconds(e as i64)
        });

        if let Some(t) = &mut self.token {
            t.access_token = access_token;
            if let Some(rt) = token_resp.refresh_token {
                t.refresh_token = Some(rt);
            }
            t.expires_at = expires_at;
        }

        self.save_token()?;
        Ok(())
    }
}

#[async_trait]
impl Authenticator for KimiOAuth {
    fn id(&self) -> &str {
        "kimi-oauth"
    }

    fn is_authenticated(&self) -> bool {
        self.token.is_some()
    }

    fn needs_refresh(&self) -> bool {
        if let Some(t) = &self.token {
            t.status(300) != crate::auth::types::TokenStatus::Valid
        } else {
            true // 没有 token 时需要触发登录
        }
    }

    async fn refresh(&mut self) -> anyhow::Result<()> {
        if !self.is_authenticated() {
            return self.do_login().await;
        }

        // 尝试刷新，失败则重新登录
        match self.do_refresh().await {
            Ok(()) => Ok(()),
            Err(_) => self.do_login().await,
        }
    }

    fn inject(&self, req: RequestBuilder) -> anyhow::Result<RequestBuilder> {
        if let Some(t) = &self.token {
            Ok(req
                .bearer_auth(&t.access_token)
                .header("User-Agent", KIMI_USER_AGENT)
                .header("X-Msh-Platform", KIMI_PLATFORM)
                .header("X-Msh-Version", KIMI_VERSION)
                .header("X-Msh-Device-Name", Self::hostname())
                .header("X-Msh-Device-Model", Self::device_model())
                .header("X-Msh-Device-Id", &self.device_id)
            )
        } else {
            Err(anyhow::anyhow!("Kimi 未认证"))
        }
    }

    fn auth_type(&self) -> AuthType {
        AuthType::OAuth
    }

    fn get_extra(&self) -> Option<&HashMap<String, serde_json::Value>> {
        self.token.as_ref().map(|t| &t.extra)
    }
}
