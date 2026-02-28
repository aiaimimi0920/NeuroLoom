use crate::site::context::{Action, UrlContext};
use crate::site::traits::Site;
use std::collections::HashMap;
use std::time::Duration;

/// Kimi (Moonshot AI) API 站点
///
/// API 端点: `https://api.kimi.com/v1`
/// 协议: OpenAI 兼容
/// 认证: Bearer Token + X-Msh-* Headers
pub struct KimiSite {
    base_url: String,
    timeout: Duration,
}

impl KimiSite {
    pub fn new() -> Self {
        Self {
            base_url: "https://api.kimi.com/v1".to_string(),
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

    fn build_url(&self, context: &UrlContext) -> String {
        let path = match context.action {
            Action::Generate => "/chat/completions",
            Action::Stream => "/chat/completions",
            Action::Embed => "/embeddings",
            Action::ImageGenerate => "/images/generations",
        };

        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }

    fn extra_headers(&self) -> HashMap<&str, &str> {
        HashMap::new()
    }

    fn timeout(&self) -> Duration {
        self.timeout
    }
}
