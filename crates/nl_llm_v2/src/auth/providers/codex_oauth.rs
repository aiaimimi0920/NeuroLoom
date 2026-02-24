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

const CODEX_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const CODEX_AUTH_URL: &str = "https://auth.openai.com/oauth/authorize";
const CODEX_TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
const CODEX_REDIRECT_URI: &str = "http://localhost:1455/auth/callback";
const CODEX_SCOPE: &str = "openid email profile offline_access";

/// Codex 特殊 Header
const CODEX_ORIGINATOR: &str = "codex_cli_rs";

// ── 响应类型 ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    id_token: Option<String>,
    expires_in: Option<u64>,
    #[allow(dead_code)]
    token_type: Option<String>,
}

// ── CodexOAuth 认证器 ─────────────────────────────────────────────────────────

/// OpenAI Codex OAuth 认证器 (Authorization Code + PKCE)
///
/// 授权流程：
/// 1. 生成 PKCE code_verifier (96 bytes) / code_challenge (S256) 和随机 state
/// 2. 本地启动 HTTP 服务器监听 :1455
/// 3. 打开浏览器至 auth.openai.com/oauth/authorize
/// 4. 等待浏览器回调，校验 state，提取 code
/// 5. POST form-urlencoded 至 auth.openai.com/oauth/token 换取 token
///
/// API 调用时注入：
/// - `Authorization: Bearer {access_token}`
/// - `Originator: codex_cli_rs`
/// - `Chatgpt-Account-Id: {account_id}`（如有）
pub struct CodexOAuth {
    token: Option<TokenStorage>,
    cache_path: Option<PathBuf>,
    callback_port: u16,
    http: Client,
}

impl CodexOAuth {
    /// 创建新的 Codex OAuth 认证器
    pub fn new(cache_path: impl AsRef<std::path::Path>) -> Self {
        let cache_path = cache_path.as_ref().to_path_buf();

        let mut auth = Self {
            token: None,
            cache_path: Some(cache_path.clone()),
            callback_port: 1455,
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

    /// 自定义回调端口（默认 1455）
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

    // ── PKCE 生成（96字节 verifier，符合 Codex 规范） ─────────────────────

    fn generate_code_verifier() -> String {
        use rand::RngCore;
        let mut bytes = [0u8; 96]; // Codex 使用 96 字节（对比 Claude 的 32 字节）
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
        let redirect_uri = if port == 1455 {
            CODEX_REDIRECT_URI.to_string()
        } else {
            format!("http://localhost:{}/auth/callback", port)
        };

        let params = [
            ("client_id", CODEX_CLIENT_ID),
            ("response_type", "code"),
            ("redirect_uri", &redirect_uri),
            ("scope", CODEX_SCOPE),
            ("state", state),
            ("code_challenge", code_challenge),
            ("code_challenge_method", "S256"),
            ("prompt", "login"),
            ("id_token_add_organizations", "true"),
            ("codex_cli_simplified_flow", "true"),
        ];

        let query = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, url_encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        format!("{}?{}", CODEX_AUTH_URL, query)
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
                return Err(anyhow::anyhow!("等待 Codex 授权超时（5 分钟）"));
            }

            let request = server.recv_timeout(Duration::from_secs(1));

            let request = match request {
                Ok(Some(r)) => r,
                Ok(None) => continue,
                Err(e) => return Err(anyhow::anyhow!("回调服务器错误: {}", e)),
            };

            let url = request.url().to_string();

            // 发送成功页面
            let html = r#"<!DOCTYPE html>
<html><head><title>Authorization Complete</title></head>
<body style="font-family:sans-serif;text-align:center;padding:50px">
<h2>✅ Codex 授权成功！</h2>
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
                    return Err(anyhow::anyhow!("Codex 授权失败: {} - {}", error, desc));
                }

                if let (Some(code), Some(state)) = (params.get("code"), params.get("state")) {
                    return Ok((code.clone(), state.clone()));
                }
            }

            return Err(anyhow::anyhow!("回调 URL 参数解析失败: {}", url));
        }
    }

    // ── Token 交换（form-urlencoded，不是 JSON） ─────────────────────────

    async fn exchange_code_for_token(
        &self,
        code: &str,
        code_verifier: &str,
        port: u16,
    ) -> anyhow::Result<TokenStorage> {
        let redirect_uri = if port == 1455 {
            CODEX_REDIRECT_URI.to_string()
        } else {
            format!("http://localhost:{}/auth/callback", port)
        };

        // Codex 使用 form-urlencoded（不是 JSON）
        let params = [
            ("grant_type", "authorization_code"),
            ("client_id", CODEX_CLIENT_ID),
            ("code", code),
            ("redirect_uri", &redirect_uri),
            ("code_verifier", code_verifier),
        ];

        let resp = self
            .http
            .post(CODEX_TOKEN_URL)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Codex token 交换失败 ({}): {}",
                status,
                body
            ));
        }

        let token_resp: TokenResponse = resp.json().await?;
        self.build_token_storage(token_resp)
    }

    // ── Token 刷新 ────────────────────────────────────────────────────────

    async fn do_refresh(&self, refresh_token: &str) -> anyhow::Result<TokenStorage> {
        let params = [
            ("client_id", CODEX_CLIENT_ID),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("scope", "openid profile email"),
        ];

        let resp = self
            .http
            .post(CODEX_TOKEN_URL)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Codex token 刷新失败 ({}): {}",
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

        // 尝试从 JWT id_token 解析 email 和 account_id
        let (email, account_id) = if let Some(ref id_token) = token_resp.id_token {
            parse_jwt_claims(id_token)
        } else {
            (None, None)
        };

        let mut extra = HashMap::new();
        if let Some(ref e) = email {
            extra.insert("email".to_string(), serde_json::Value::String(e.clone()));
        }
        if let Some(ref aid) = account_id {
            extra.insert("account_id".to_string(), serde_json::Value::String(aid.clone()));
        }
        if let Some(ref id_token) = token_resp.id_token {
            extra.insert("id_token".to_string(), serde_json::Value::String(id_token.clone()));
        }

        Ok(TokenStorage {
            access_token: token_resp.access_token,
            refresh_token: token_resp.refresh_token,
            expires_at,
            email,
            provider: "codex_oauth".to_string(),
            extra,
        })
    }

    // ── 完整登录流程 ──────────────────────────────────────────────────────

    async fn do_login(&mut self) -> anyhow::Result<()> {
        println!("\n=== OpenAI Codex OAuth Login ===");

        // 1. 生成 PKCE + State
        let code_verifier = Self::generate_code_verifier();
        let code_challenge = Self::generate_code_challenge(&code_verifier);
        let state = Self::generate_state();

        // 2. 构建授权 URL
        let auth_url = Self::build_auth_url(&state, &code_challenge, self.callback_port);

        // 3. 提示用户
        println!("请在浏览器中完成 Codex 授权:");
        println!("{}\n", auth_url);

        let _ = open::that(&auth_url);

        // 4. 等待回调
        println!("等待浏览器回调 (localhost:{})...", self.callback_port);
        let port = self.callback_port;
        let timeout = Duration::from_secs(300);

        let callback_result = tokio::task::spawn_blocking(move || {
            Self::wait_for_callback(port, timeout)
        })
        .await
        .map_err(|e| anyhow::anyhow!("回调等待线程失败: {}", e))??;

        let (code, returned_state) = callback_result;

        // 5. 校验 State
        if returned_state != state {
            return Err(anyhow::anyhow!(
                "State 校验失败，expected={}, got={}",
                &state[..8.min(state.len())],
                &returned_state[..8.min(returned_state.len())]
            ));
        }

        println!("Codex 授权成功！正在交换 token...");

        // 6. 交换 code → token
        let token = self
            .exchange_code_for_token(&code, &code_verifier, self.callback_port)
            .await?;

        if let Some(ref email) = token.email {
            println!("已登录账户: {}", email);
        }
        println!("✅ Codex OAuth 登录完成！");

        self.token = Some(token);
        self.save_token()?;
        Ok(())
    }
}

// ── Authenticator 实现 ───────────────────────────────────────────────────────

#[async_trait]
impl Authenticator for CodexOAuth {
    fn id(&self) -> &str {
        "codex_oauth"
    }

    fn is_authenticated(&self) -> bool {
        self.token.is_some()
    }

    fn needs_refresh(&self) -> bool {
        if let Some(t) = &self.token {
            // 提前 5 天刷新（与 CLIProxyAPI 参考一致：RefreshLead = 5 * 24h）
            t.status(5 * 24 * 3600) != TokenStatus::Valid
        } else {
            true
        }
    }

    async fn refresh(&mut self) -> anyhow::Result<()> {
        if !self.is_authenticated() {
            return self.do_login().await;
        }

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
                self.do_login().await
            }
        }
    }

    fn inject(&self, req: RequestBuilder) -> anyhow::Result<RequestBuilder> {
        if let Some(t) = &self.token {
            // 生成随机 session_id（替代 uuid）
            let session_id = {
                use rand::RngCore;
                let mut bytes = [0u8; 16];
                rand::thread_rng().fill_bytes(&mut bytes);
                bytes.iter().map(|b| format!("{:02x}", b)).collect::<String>()
            };

            let mut req = req.bearer_auth(&t.access_token)
                .header("Originator", CODEX_ORIGINATOR)
                .header("User-Agent", "codex_cli_rs/0.101.0 (Windows; x86_64)")
                .header("Session_id", &session_id);

            // 注入 Chatgpt-Account-Id（如果有）
            if let Some(account_id) = t.extra.get("account_id").and_then(|v| v.as_str()) {
                req = req.header("Chatgpt-Account-Id", account_id);
            }

            Ok(req)
        } else {
            Err(anyhow::anyhow!("Codex 未认证"))
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

// ── JWT 解析（简单解析 payload，不验证签名） ──────────────────────────────────

/// 从 JWT id_token 解析 email 和 account_id（不验证签名）
fn parse_jwt_claims(token: &str) -> (Option<String>, Option<String>) {
    use base64::Engine;

    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() < 2 {
        return (None, None);
    }

    // JWT payload 是第二部分
    let payload = parts[1];
    // 添加 padding
    let padded = match payload.len() % 4 {
        2 => format!("{}==", payload),
        3 => format!("{}=", payload),
        _ => payload.to_string(),
    };

    let decoded = match base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(padded.trim_end_matches('='))
    {
        Ok(d) => d,
        Err(_) => return (None, None),
    };

    let json: serde_json::Value = match serde_json::from_slice(&decoded) {
        Ok(v) => v,
        Err(_) => return (None, None),
    };

    let email = json.get("email").and_then(|v| v.as_str()).map(|s| s.to_string());

    // account_id 可能在 codex_auth_info.chatgpt_account_id 或直接 sub 字段
    let account_id = json
        .get("codex_auth_info")
        .and_then(|info| info.get("chatgpt_account_id"))
        .and_then(|v| v.as_str())
        .or_else(|| json.get("sub").and_then(|v| v.as_str()))
        .map(|s| s.to_string());

    (email, account_id)
}

// ── 工具函数 ─────────────────────────────────────────────────────────────────

fn base64_url_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}

fn parse_query(query: &str) -> HashMap<String, String> {
    query
        .split('&')
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let key = parts.next()?.to_string();
            let value = parts.next().unwrap_or("").replace('+', " ");
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
