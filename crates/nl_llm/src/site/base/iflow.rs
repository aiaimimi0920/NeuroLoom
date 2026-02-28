use crate::site::context::UrlContext;
use crate::site::traits::Site;
use std::collections::HashMap;
use std::time::Duration;

/// iFlow 网关平台实现
///
/// iFlow 是一个 OpenAI 兼容的 Chat Completions 网关，
/// 不按 Action 区分端点（所有请求都发送到 /v1/chat/completions）
pub struct IFlowSite {
    base_url: String,
    timeout: Duration,
}

impl IFlowSite {
    pub fn new() -> Self {
        Self {
            base_url: "https://apis.iflow.cn/v1/chat/completions".to_string(),
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

impl Default for IFlowSite {
    fn default() -> Self {
        Self::new()
    }
}

impl Site for IFlowSite {
    fn id(&self) -> &str {
        "iflow"
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn build_url(&self, _ctx: &UrlContext) -> String {
        // iFlow 只提供单纯的 Completions 网关，不按动作区分
        self.base_url.clone()
    }

    fn extra_headers(&self) -> HashMap<&str, &str> {
        let mut headers = HashMap::new();
        headers.insert("Content-Type", "application/json");
        // 伪装成真实的 iFlow CLI
        headers.insert("User-Agent", "iFlow-Cli");
        headers
    }

    fn timeout(&self) -> Duration {
        self.timeout
    }
}
