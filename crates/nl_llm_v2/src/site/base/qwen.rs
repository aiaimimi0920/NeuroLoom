use crate::site::context::UrlContext;
use crate::site::traits::Site;
use std::collections::HashMap;
use std::time::Duration;

/// Qwen OAuth 模式网关
///
/// API 端点: `https://portal.qwen.ai/v1/chat/completions`
/// 协议: OpenAI 兼容
/// 认证: Bearer Token (OAuth access_token) + X-Dashscope-Authtype: qwen-oauth
pub struct QwenSite {
    base_url: String,
    timeout: Duration,
}

impl QwenSite {
    pub fn new() -> Self {
        Self {
            base_url: "https://portal.qwen.ai/v1/chat/completions".to_string(),
            timeout: Duration::from_secs(120),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

impl Default for QwenSite {
    fn default() -> Self {
        Self::new()
    }
}

impl Site for QwenSite {
    fn id(&self) -> &str {
        "qwen"
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn build_url(&self, _ctx: &UrlContext) -> String {
        // Qwen 走 OpenAI 兼容端点，不按 Action 区分
        self.base_url.clone()
    }

    fn extra_headers(&self) -> HashMap<&str, &str> {
        HashMap::new()
    }

    fn timeout(&self) -> Duration {
        self.timeout
    }
}
