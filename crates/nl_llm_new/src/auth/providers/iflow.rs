//! IFlow (Cookie -> API Key) 认证实现
//!
//! iFlow 使用 Cookie 认证获取临时 API Key，与标准 OAuth 流程不同：
//! - 无需浏览器登录，Cookie 由用户提供
//! - API Key 通过两步获取：GET 获取信息 → POST 刷新获取完整 Key
//! - API Key 无明确过期时间，但建议缓存复用

use crate::auth::{AuthError, TokenStatus};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::Deserialize;
use std::time::Duration;

/// iFlow 认证客户端
///
/// 负责 Cookie → API Key 的获取和缓存
pub struct IFlowAuth {
    /// 用户提供的 Cookie (BXAuth=...)
    cookie: String,
    /// 缓存的 API Key
    cached_api_key: Option<String>,
    /// 复用的 HTTP Client
    http: reqwest::Client,
}

impl IFlowAuth {
    /// 创建新的 IFlow 认证实例
    pub fn new(cookie: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client for IFlowAuth");

        Self {
            cookie,
            cached_api_key: None,
            http,
        }
    }

    /// 从 Cookie 字符串创建
    pub fn from_cookie(cookie: &str) -> Result<Self, AuthError> {
        Ok(Self::new(cookie.to_string()))
    }

    /// 获取 API Key（带缓存）
    ///
    /// 如果已有缓存的 API Key，直接返回；
    /// 否则通过 Cookie 获取新的 API Key
    pub async fn get_api_key(&mut self) -> Result<&str, AuthError> {
        if self.cached_api_key.is_some() {
            return Ok(self.cached_api_key.as_ref().unwrap());
        }

        self.fetch_api_key().await
    }

    /// 获取当前缓���的 API Key（不触发刷新）
    pub fn cached_key(&self) -> Option<&str> {
        self.cached_api_key.as_deref()
    }

    /// 强制刷新 API Key
    ///
    /// 执行 GET + POST 两步获取新的 API Key
    pub async fn fetch_api_key(&mut self) -> Result<&str, AuthError> {
        // 1. GET 获取基础信息
        let get_resp = self
            .http
            .get("https://platform.iflow.cn/api/openapi/apikey")
            .headers(Self::build_headers(&self.cookie, false))
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
            .headers(Self::build_headers(&self.cookie, true))
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

        self.cached_api_key = Some(post_data.api_key);
        Ok(self.cached_api_key.as_ref().unwrap())
    }

    /// 清除缓存的 API Key
    ///
    /// 当 Cookie 失效或需要强制刷新时调用
    pub fn clear_cache(&mut self) {
        self.cached_api_key = None;
    }

    /// 检查是否有缓存的 API Key
    pub fn has_cache(&self) -> bool {
        self.cached_api_key.is_some()
    }

    /// 构建请求头
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

    /// 获取 Cookie 引用
    pub fn cookie(&self) -> &str {
        &self.cookie
    }
}

/// iFlow Token 状态（兼容 TokenStorage trait）
impl IFlowAuth {
    /// iFlow API Key 无明确过期时间，始终返回 Valid
    pub fn status(&self) -> TokenStatus {
        if self.cached_api_key.is_some() {
            TokenStatus::Valid
        } else {
            TokenStatus::Expired
        }
    }

    /// 是否需要获取 API Key
    pub fn needs_fetch(&self) -> bool {
        self.cached_api_key.is_none()
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
        let auth = IFlowAuth::new("BXAuth=test".to_string());
        assert!(auth.needs_fetch());
        assert!(!auth.has_cache());
    }

    #[test]
    fn test_status_without_cache() {
        let auth = IFlowAuth::new("BXAuth=test".to_string());
        assert_eq!(auth.status(), TokenStatus::Expired);
    }
}
