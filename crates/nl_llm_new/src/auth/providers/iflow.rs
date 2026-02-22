//! IFlow (Cookie -> API Key) 认证实现
//!
//! iFlow 使用 Cookie 认证获取临时 API Key，与标准 OAuth 流程不同：
//! - 无需浏览器登录，Cookie 由用户提供
//! - API Key 通过两步获取：GET 获取信息 → POST 刷新获取完整 Key
//! - API Key 有过期时间，支持持久化缓存复用

use crate::auth::{AuthError, TokenStatus, TokenStorage};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// iFlow 认证客户端
///
/// 负责 Cookie → API Key 的获取和持久化缓存
pub struct IFlowAuth {
    /// Token 文件路径
    path: Option<PathBuf>,
    /// 当前 Token（使用统一 TokenStorage）
    pub token: Option<TokenStorage>,
    /// 复用的 HTTP Client
    http: reqwest::Client,
}

impl IFlowAuth {
    /// 创建新的认证客户端（内存模式）
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client for IFlowAuth");

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
            .expect("Failed to create HTTP client for IFlowAuth");

        if path.exists() {
            let content = std::fs::read_to_string(path).unwrap_or_default();
            match serde_json::from_str::<TokenStorage>(&content) {
                Ok(token) => Ok(Self {
                    path: Some(path.to_path_buf()),
                    token: Some(token),
                    http,
                }),
                Err(e) => {
                    eprintln!("Warning: Failed to parse token file: {}. Will fetch new cookie.", e);
                    Ok(Self {
                        path: Some(path.to_path_buf()),
                        token: None,
                        http,
                    })
                }
            }
        } else {
            Ok(Self {
                path: Some(path.to_path_buf()),
                token: None,
                http,
            })
        }
    }

    /// 从 Cookie 字符串创建（内存模式）
    ///
    /// 用于用户直接提供 Cookie 的场景，不进行文件持久化
    pub fn from_cookie(cookie: &str) -> Result<Self, AuthError> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client for IFlowAuth");

        // 提取 BXAuth 部分
        let bx_auth = Self::extract_bx_auth(cookie);

        let mut extra = std::collections::HashMap::new();
        extra.insert("cookie".to_string(), serde_json::Value::String(bx_auth));

        Ok(Self {
            path: None,
            token: Some(TokenStorage {
                access_token: String::new(),
                refresh_token: None,
                expires_at: None,
                email: None,
                provider: "IFlow".to_string(),
                extra,
            }),
            http,
        })
    }

    /// 设置 Cookie（用于后续获取 API Key）
    pub fn set_cookie(&mut self, cookie: &str) {
        let bx_auth = Self::extract_bx_auth(cookie);

        if let Some(ref mut token) = self.token {
            token.extra.insert("cookie".to_string(), serde_json::Value::String(bx_auth));
        } else {
            let mut extra = std::collections::HashMap::new();
            extra.insert("cookie".to_string(), serde_json::Value::String(bx_auth));
            self.token = Some(TokenStorage {
                access_token: String::new(),
                refresh_token: None,
                expires_at: None,
                email: None,
                provider: "IFlow".to_string(),
                extra,
            });
        }
    }

    /// 确保已认证（获取 API Key）
    pub async fn ensure_authenticated(&mut self) -> Result<(), AuthError> {
        // 检查是否有 Cookie
        let has_cookie = self.token.as_ref().map_or(false, |t| {
            t.extra.get("cookie").and_then(|v| v.as_str()).map_or(false, |s| !s.is_empty())
        });

        if !has_cookie {
            return Err(AuthError::InvalidCredentials("No cookie provided".to_string()));
        }

        // 检查是否需要获取/刷新 API Key
        let needs_fetch = self.needs_refresh();

        if needs_fetch {
            let api_key = self.fetch_api_key_static().await?;

            if let Some(ref mut token) = self.token {
                token.access_token = api_key.clone();
                // 更新过期时间（假设 7 天有效）
                token.expires_at = Some(chrono::Utc::now() + chrono::Duration::days(7));

                if let Some(ref path) = self.path {
                    let _ = Self::save_token_to_path_static(token, path);
                }
            }
        }

        Ok(())
    }

    /// 获取 Token 状态
    pub fn token_status(&self) -> TokenStatus {
        self.token.as_ref().map_or(TokenStatus::Expired, |t| {
            // 检查是否有 API Key
            if t.access_token.is_empty() {
                return TokenStatus::Expired;
            }
            // 使用 7 天的提前量检查过期
            t.status(7 * 24 * 3600)
        })
    }

    /// 检查是否需要刷新
    pub fn needs_refresh(&self) -> bool {
        matches!(self.token_status(), TokenStatus::Expired | TokenStatus::ExpiringSoon)
    }

    /// 获取 API Key
    pub fn api_key(&self) -> Option<&str> {
        self.token.as_ref().map(|t| t.access_token.as_str()).filter(|s| !s.is_empty())
    }

    /// 获取 Cookie
    pub fn cookie(&self) -> Option<&str> {
        self.token.as_ref().and_then(|t| t.extra.get("cookie")?.as_str())
    }

    /// 强制刷新 API Key
    pub async fn fetch_api_key(&mut self) -> Result<String, AuthError> {
        let api_key = self.fetch_api_key_static().await?;

        if let Some(ref mut token) = self.token {
            token.access_token = api_key.clone();
            token.expires_at = Some(chrono::Utc::now() + chrono::Duration::days(7));

            if let Some(ref path) = self.path {
                let _ = Self::save_token_to_path_static(token, path);
            }
        }

        Ok(api_key)
    }

    /// 获取 API Key（静态方法，供内部调用）
    async fn fetch_api_key_static(&self) -> Result<String, AuthError> {
        let cookie = self.cookie().ok_or_else(|| {
            AuthError::InvalidCredentials("No cookie available".to_string())
        })?;

        // 1. GET 获取基础信息
        let get_resp = self
            .http
            .get("https://platform.iflow.cn/api/openapi/apikey")
            .headers(Self::build_headers(cookie, false))
            .send()
            .await
            .map_err(|e| AuthError::Http(format!("GET apikey failed: {}", e)))?;

        if !get_resp.status().is_success() {
            return Err(AuthError::Http(format!(
                "GET apikey returned status {}",
                get_resp.status()
            )));
        }

        let json_get: IFlowApiKeyResponse = get_resp.json().await.map_err(|e| {
            AuthError::Http(format!("Failed to parse GET response: {}", e))
        })?;

        if !json_get.success {
            return Err(AuthError::InvalidCredentials(
                "GET apikey response success=false".to_string(),
            ));
        }

        let data = json_get.data.ok_or_else(|| {
            AuthError::InvalidCredentials("Missing data in GET response".to_string())
        })?;

        // 2. POST 刷新获取完整 API Key
        let post_body = serde_json::json!({ "name": data.name });

        let post_resp = self
            .http
            .post("https://platform.iflow.cn/api/openapi/apikey")
            .headers(Self::build_headers(cookie, true))
            .json(&post_body)
            .send()
            .await
            .map_err(|e| AuthError::Http(format!("POST apikey failed: {}", e)))?;

        if !post_resp.status().is_success() {
            return Err(AuthError::Http(format!(
                "POST apikey returned status {}",
                post_resp.status()
            )));
        }

        let json_post: IFlowApiKeyResponse = post_resp.json().await.map_err(|e| {
            AuthError::Http(format!("Failed to parse POST response: {}", e))
        })?;

        if !json_post.success {
            return Err(AuthError::RefreshFailed(
                "POST apikey response success=false".to_string(),
            ));
        }

        let post_data = json_post.data.ok_or_else(|| {
            AuthError::RefreshFailed("Missing data in POST response".to_string())
        })?;

        Ok(post_data.api_key)
    }

    /// 构建 API 请求头
    fn build_headers(cookie: &str, is_post: bool) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Cookie",
            HeaderValue::from_str(cookie).unwrap_or_else(|_| HeaderValue::from_static("")),
        );
        headers.insert(
            "Accept",
            HeaderValue::from_static("application/json, text/plain, */*"),
        );
        headers.insert(
            "User-Agent",
            HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/120.0.0.0 Safari/537.36",
            ),
        );
        headers.insert("Sec-Fetch-Dest", HeaderValue::from_static("empty"));
        headers.insert("Sec-Fetch-Mode", HeaderValue::from_static("cors"));
        headers.insert("Sec-Fetch-Site", HeaderValue::from_static("same-origin"));

        if is_post {
            headers.insert("Content-Type", HeaderValue::from_static("application/json"));
            headers.insert("Origin", HeaderValue::from_static("https://platform.iflow.cn"));
            headers.insert("Referer", HeaderValue::from_static("https://platform.iflow.cn/"));
        }

        headers
    }

    /// 从 Cookie 字符串中提取 BXAuth 部分
    fn extract_bx_auth(cookie: &str) -> String {
        // 尝试提取 BXAuth=xxx 格式
        for part in cookie.split(';') {
            let part = part.trim();
            if part.starts_with("BXAuth=") {
                return format!("{};", part);
            }
        }
        // 如果没有找到 BXAuth，返回整个 cookie
        if !cookie.is_empty() && !cookie.ends_with(';') {
            format!("{};", cookie)
        } else {
            cookie.to_string()
        }
    }

    /// 保存 Token 到文件（静态方法）
    fn save_token_to_path_static(
        token: &TokenStorage,
        path: &Path,
    ) -> Result<(), std::io::Error> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(token)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// 清除缓存的 API Key
    pub fn clear_cache(&mut self) {
        if let Some(ref mut token) = self.token {
            token.access_token = String::new();
            token.expires_at = None;
        }
    }

    /// 检查是否有缓存的 API Key
    pub fn has_cache(&self) -> bool {
        self.api_key().is_some()
    }
}

impl Default for IFlowAuth {
    fn default() -> Self {
        Self::new()
    }
}

// ==================== 内部结构 ====================

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iflow_auth_new() {
        let auth = IFlowAuth::new();
        assert!(auth.needs_refresh());
        assert!(!auth.has_cache());
    }

    #[test]
    fn test_from_cookie() {
        let auth = IFlowAuth::from_cookie("BXAuth=test123; other=value").unwrap();
        assert_eq!(auth.cookie(), Some("BXAuth=test123;"));
    }

    #[test]
    fn test_extract_bx_auth() {
        assert_eq!(
            IFlowAuth::extract_bx_auth("BXAuth=abc123; other=value"),
            "BXAuth=abc123;"
        );
        assert_eq!(
            IFlowAuth::extract_bx_auth("other=value; BXAuth=xyz789;"),
            "BXAuth=xyz789;"
        );
        assert_eq!(
            IFlowAuth::extract_bx_auth("some_cookie_value"),
            "some_cookie_value;"
        );
    }

    #[test]
    fn test_status_without_cache() {
        let auth = IFlowAuth::new();
        assert_eq!(auth.token_status(), TokenStatus::Expired);
    }
}
