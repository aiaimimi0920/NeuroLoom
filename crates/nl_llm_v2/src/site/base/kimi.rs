use std::collections::HashMap;
use std::time::Duration;
use crate::site::traits::Site;
use crate::site::context::UrlContext;

/// Kimi (Moonshot AI) API 站点
///
/// API 端点: `https://api.kimi.com/coding/v1/chat/completions`
/// 协议: OpenAI 兼容
/// 认证: Bearer Token + X-Msh-* Headers
pub struct KimiSite {
    base_url: String,
    timeout: Duration,
}

impl KimiSite {
    pub fn new() -> Self {
        Self {
            base_url: "https://api.kimi.com/coding/v1/chat/completions".to_string(),
            timeout: Duration::from_secs(120),
        }
    }
}

impl Default for KimiSite {
    fn default() -> Self {
        Self::new()
    }
}

impl Site for KimiSite {
    fn id(&self) -> &str {
        "kimi"
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn build_url(&self, _ctx: &UrlContext) -> String {
        self.base_url.clone()
    }

    fn extra_headers(&self) -> HashMap<&str, &str> {
        HashMap::new()
    }

    fn timeout(&self) -> Duration {
        self.timeout
    }
}
