use std::collections::HashMap;
use std::time::Duration;
use crate::site::traits::Site;
use crate::site::context::UrlContext;

/// iFlow 网关平台实现
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
        headers.insert("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36");
        headers
    }

    fn timeout(&self) -> Duration {
        self.timeout
    }
}
