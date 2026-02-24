use async_trait::async_trait;
use reqwest::{Client, RequestBuilder};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use crate::auth::traits::Authenticator;
use crate::auth::types::{TokenStatus, TokenStorage};
use crate::site::context::AuthType;

// ── OAuth 常量 ────────────────────────────────────────────────────────────────

const ANTHROPIC_CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
const ANTHROPIC_AUTH_URL: &str = "https://claude.ai/oauth/authorize";
const ANTHROPIC_TOKEN_URL: &str = "https://console.anthropic.com/v1/oauth/token";
const ANTHROPIC_REDIRECT_URI: &str = "http://localhost:54545/callback";
const ANTHROPIC_SCOPE: &str = "org:create_api_key user:profile user:inference";

/// Anthropic 注入的 Header
#[allow(dead_code)]
const ANTHROPIC_VERSION: &str = "2023-06-01"; // AnthropicSite 已注入，保留备用
const ANTHROPIC_OAUTH_BETA: &str = "oauth-2025-04-20";
const ANTHROPIC_BETA_HEADER: &str = "anthropic-beta";

// ── 响应类型 ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
    #[allow(dead_code)]
    token_type: Option<String>,

    // 账户信息（可能不在所有响应中）
    #[serde(default)]
    account: AccountInfo,
}

#[derive(Debug, Deserialize, Default)]
struct AccountInfo {
    #[serde(rename = "email_address")]
    email_address: Option<String>,
}

// ── AnthropicOAuth 认证器 ────────────────────────────────────────────────────

/// Anthropic Claude OAuth 认证器 (Authorization Code + PKCE)
///
/// 授权流程：
/// 1. 生成 PKCE code_verifier / code_challenge (S256) 和随机 state
/// 2. 本地启动 HTTP 服务器监听 :54545
/// 3. 打开浏览器至 claude.ai/oauth/authorize
/// 4. 等待浏览器回调，校验 state，提取 code
/// 5. POST JSON 至 console.anthropic.com/v1/oauth/token 换取 token
///
/// API 调用时注入：
/// - `Authorization: Bearer {access_token}`
/// - `anthropic-version: 2023-06-01`
/// - `anthropic-beta: oauth-2025-04-20`
pub struct AnthropicOAuth {
    token: Option<TokenStorage>,
    cache_path: Option<PathBuf>,
    callback_port: u16,
    http: Client,
}

impl AnthropicOAuth {
    /// 创建新的 Anthropic OAuth 认证器
    pub fn new(cache_path: impl AsRef<std::path::Path>) -> Self {
        let cache_path = cache_path.as_ref().to_path_buf();

        let mut auth = Self {
            token: None,
            cache_path: Some(cache_path.clone()),
            callback_port: 54545,
            http: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
        };

        // 尝试从缓存加载 token
        if let Ok(content) = std::fs::read_to_string(&cache_path) {
            if let Ok(t) = serde_json::from_str::<TokenStorage>(&content) {
                auth.token = Some(t);
            }
        }

        auth
    }

    /// 自定义回调端口（默认 54545）
    pub fn with_callback_port(mut self, port: u16) -> Self {
        self.callback_port = port;
        self
    }

    /// 保存 token 到缓存文件
    fn save_token(&self) -> anyhow::Result<()> {
        if let (Some(t), Some(path)) = (&self.token, &self.cache_path) {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, serde_json::to_string_pretty(t)?)?;
        }
        Ok(())
    }

    // ── PKCE 生成 ─────────────────────────────────────────────────────────

    fn generate_code_verifier() -> String {
        use rand::RngCore;
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        base64_url_encode(&bytes)
    }

    fn generate_code_challenge(verifier: &str) -> String {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(verifier.as_bytes());
        base64_url_encode(&hash)
    }

    fn generate_state() -> String {
        use rand::RngCore;
        let mut bytes = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut bytes);
        base64_url_encode(&bytes)
    }

    // ── 构建授权 URL ──────────────────────────────────────────────────────

    fn build_auth_url(state: &str, code_challenge: &str, port: u16) -> String {
        let redirect_uri = if port == 54545 {
            ANTHROPIC_REDIRECT_URI.to_string()
        } else {
            format!("http://localhost:{}/callback", port)
        };

        let params = [
            ("code", "true"),
            ("client_id", ANTHROPIC_CLIENT_ID),
            ("response_type", "code"),
            ("redirect_uri", &redirect_uri),
            ("scope", ANTHROPIC_SCOPE),
            ("code_challenge", code_challenge),
            ("code_challenge_method", "S256"),
            ("state", state),
        ];

        let query = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, url_encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        format!("{}?{}", ANTHROPIC_AUTH_URL, query)
    }

    // ── 本地回调服务器（tiny_http） ────────────────────────────────────────

    /// 启动本地 HTTP 服务器，等待 OAuth 回调，返回 (code, state)
    fn wait_for_callback(port: u16, timeout: Duration) -> anyhow::Result<(String, String)> {
        let addr = format!("0.0.0.0:{}", port);
        let server = tiny_http::Server::http(&addr)
            .map_err(|e| anyhow::anyhow!("无法启动本地 OAuth 回调服务器 ({}): {}", addr, e))?;

        let deadline = std::time::Instant::now() + timeout;

        loop {
            if std::time::Instant::now() > deadline {
                return Err(anyhow::anyhow!("等待 Anthropic 授权超时（5 分钟）"));
            }

            // 非阻塞等待（超时 1 秒轮询一次）
            let request = server.recv_timeout(Duration::from_secs(1));

            let request = match request {
                Ok(Some(r)) => r,
                Ok(None) => continue, // 超时继续等待
                Err(e) => return Err(anyhow::anyhow!("回调服务器错误: {}", e)),
            };

            let url = request.url().to_string();

            // 发送友好的成功页面给浏览器
            let html = r#"<!DOCTYPE html>
<html><head><title>授权完成</title></head>
<body style="font-family:sans-serif;text-align:center;padding:50px">
<h2>✅ Anthropic 授权成功！</h2>
<p>您可以关闭此窗口返回程序。</p>
</body></html>"#;

            let resp = tiny_http::Response::from_string(html)
                .with_header(
                    tiny_http::Header::from_bytes("Content-Type", "text/html; charset=utf-8")
                        .unwrap(),
                );
            let _ = request.respond(resp);

            // 解析 URL 参数
            if let Some(query_start) = url.find('?') {
                let query = &url[query_start + 1..];
                let params = parse_query(query);

                if let Some(error) = params.get("error") {
                    let desc = params
                        .get("error_description")
                        .map(|s| s.as_str())
                        .unwrap_or("unknown");
                    return Err(anyhow::anyhow!("Anthropic 授权失败: {} - {}", error, desc));
                }

                if let (Some(code), Some(state)) = (params.get("code"), params.get("state")) {
                    return Ok((code.clone(), state.clone()));
                }
            }

            return Err(anyhow::anyhow!("回调 URL 参数解析失败: {}", url));
        }
    }

    // ── Token 交换 ────────────────────────────────────────────────────────

    async fn exchange_code_for_token(
        &self,
        code: &str,
        code_verifier: &str,
        state: &str,
        port: u16,
    ) -> anyhow::Result<TokenStorage> {
        let redirect_uri = if port == 54545 {
            ANTHROPIC_REDIRECT_URI.to_string()
        } else {
            format!("http://localhost:{}/callback", port)
        };

        let body = serde_json::json!({
            "code": code,
            "state": state,
            "grant_type": "authorization_code",
            "client_id": ANTHROPIC_CLIENT_ID,
            "redirect_uri": redirect_uri,
            "code_verifier": code_verifier,
        });

        let resp = self
            .http
            .post(ANTHROPIC_TOKEN_URL)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Anthropic token 交换失败 ({}): {}",
                status,
                body
            ));
        }

        let token_resp: TokenResponse = resp.json().await?;
        self.build_token_storage(token_resp)
    }

    // ── Token 刷新 ────────────────────────────────────────────────────────

    async fn do_refresh(&self, refresh_token: &str) -> anyhow::Result<TokenStorage> {
        let body = serde_json::json!({
            "client_id": ANTHROPIC_CLIENT_ID,
            "grant_type": "refresh_token",
            "refresh_token": refresh_token,
        });

        let resp = self
            .http
            .post(ANTHROPIC_TOKEN_URL)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Anthropic token 刷新失败 ({}): {}",
                status,
                body
            ));
        }

        let token_resp: TokenResponse = resp.json().await?;
        self.build_token_storage(token_resp)
    }

    fn build_token_storage(&self, token_resp: TokenResponse) -> anyhow::Result<TokenStorage> {
        let expires_at = token_resp.expires_in.map(|e| {
            chrono::Utc::now() + chrono::Duration::seconds(e as i64)
        });

        let email = token_resp.account.email_address.clone();

        let mut extra = HashMap::new();
        if let Some(ref e) = email {
            extra.insert("email".to_string(), serde_json::Value::String(e.clone()));
        }

        Ok(TokenStorage {
            access_token: token_resp.access_token,
            refresh_token: token_resp.refresh_token,
            expires_at,
            email,
            provider: "anthropic_oauth".to_string(),
            extra,
        })
    }

    // ── 完整登录流程 ──────────────────────────────────────────────────────

    async fn do_login(&mut self) -> anyhow::Result<()> {
        println!("\n=== Anthropic Claude OAuth Login ===");

        // 1. 生成 PKCE + State
        let code_verifier = Self::generate_code_verifier();
        let code_challenge = Self::generate_code_challenge(&code_verifier);
        let state = Self::generate_state();

        // 2. 构建授权 URL
        let auth_url = Self::build_auth_url(&state, &code_challenge, self.callback_port);

        // 3. 提示用户
        println!("请在浏览器中完成 Anthropic 授权:");
        println!("{}\n", auth_url);

        // 尝试自动打开浏览器
        let _ = open::that(&auth_url);

        // 4. 在后台线程等待回调（因为 tiny_http 是同步的）
        println!("等待浏览器回调 ({}:{})...", "localhost", self.callback_port);
        let port = self.callback_port;
        let timeout = Duration::from_secs(300); // 5 分钟超时

        let callback_result = tokio::task::spawn_blocking(move || {
            Self::wait_for_callback(port, timeout)
        })
        .await
        .map_err(|e| anyhow::anyhow!("回调等待线程失败: {}", e))??;

        let (code, returned_state) = callback_result;

        // 5. 校验 State（CSRF 防护）
        if returned_state != state {
            return Err(anyhow::anyhow!(
                "State 校验失败（可能存在 CSRF 攻击），expected={}, got={}",
                &state[..8.min(state.len())],
                &returned_state[..8.min(returned_state.len())]
            ));
        }

        println!("Anthropic 授权成功！正在交换 token...");

        // 6. 交换 code → token
        let token =
            self.exchange_code_for_token(&code, &code_verifier, &state, self.callback_port)
                .await?;

        if let Some(ref email) = token.email {
            println!("已登录账户: {}", email);
        }
        println!("✅ Anthropic OAuth 登录完成！");

        self.token = Some(token);
        self.save_token()?;
        Ok(())
    }
}

// ── Authenticator 实现 ───────────────────────────────────────────────────────

#[async_trait]
impl Authenticator for AnthropicOAuth {
    fn id(&self) -> &str {
        "anthropic_oauth"
    }

    fn is_authenticated(&self) -> bool {
        self.token.is_some()
    }

    fn needs_refresh(&self) -> bool {
        if let Some(t) = &self.token {
            // 提前 4 小时刷新（与 CLIProxyAPI 参考一致）
            t.status(14400) != TokenStatus::Valid
        } else {
            true // 无 token 时需要触发登录
        }
    }

    async fn refresh(&mut self) -> anyhow::Result<()> {
        if !self.is_authenticated() {
            return self.do_login().await;
        }

        // 尝试 refresh_token 刷新
        let refresh_result = if let Some(rt) = self
            .token
            .as_ref()
            .and_then(|t| t.refresh_token.as_deref())
        {
            let rt = rt.to_string();
            Some(self.do_refresh(&rt).await)
        } else {
            None
        };

        match refresh_result {
            Some(Ok(new_token)) => {
                self.token = Some(new_token);
                self.save_token()?;
                Ok(())
            }
            Some(Err(_)) | None => {
                // 刷新失败或无 refresh_token，重新完整登录
                self.do_login().await
            }
        }
    }

    fn inject(&self, req: RequestBuilder) -> anyhow::Result<RequestBuilder> {
        if let Some(t) = &self.token {
            // AnthropicSite.extra_headers() 已注入 anthropic-version，
            // 此处只需注入 Bearer token 和 OAuth 专用 beta header
            Ok(req
                .bearer_auth(&t.access_token)
                .header(ANTHROPIC_BETA_HEADER, ANTHROPIC_OAUTH_BETA))
        } else {
            Err(anyhow::anyhow!("Anthropic 未认证"))
        }
    }

    fn auth_type(&self) -> AuthType {
        AuthType::OAuth
    }

    fn get_extra<'a>(
        &'a self,
    ) -> Option<&'a HashMap<String, serde_json::Value>> {
        self.token.as_ref().map(|t| &t.extra)
    }
}

// ── 工具函数 ─────────────────────────────────────────────────────────────────

fn base64_url_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}

/// 简单的 URL 参数解析
fn parse_query(query: &str) -> HashMap<String, String> {
    query
        .split('&')
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let key = parts.next()?.to_string();
            let value = parts.next().unwrap_or("").replace('+', " ");
            // 简单 percent decode：替换 %XX
            Some((key, percent_decode(&value)))
        })
        .collect()
}

fn percent_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            let h1 = chars.next().unwrap_or('0');
            let h2 = chars.next().unwrap_or('0');
            if let Ok(byte) = u8::from_str_radix(&format!("{}{}", h1, h2), 16) {
                result.push(byte as char);
            } else {
                result.push('%');
                result.push(h1);
                result.push(h2);
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// 简单的 URL 参数编码（只处理常见特殊字符）
fn url_encode(s: &str) -> String {
    s.replace('%', "%25")
        .replace(' ', "%20")
        .replace('+', "%2B")
        .replace('/', "%2F")
        .replace('?', "%3F")
        .replace('&', "%26")
        .replace('=', "%3D")
        .replace('#', "%23")
        .replace(':', "%3A")
}
