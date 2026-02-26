use std::collections::HashMap;
use std::time::Duration;

use crate::site::context::{Action, UrlContext};
use crate::site::traits::Site;

/// Vidu 站点（占位）
///
/// Vidu 的核心能力主要通过 ProviderExtension 的视频任务 API 完成。
/// Site 在此仅用于满足框架的 pipeline 组装要求。
pub struct ViduSite {
    base_url: String,
    timeout: Duration,
    extra_headers: HashMap<String, String>,
}

impl ViduSite {
    pub fn new() -> Self {
        Self {
            base_url: "https://api.vidu.cn".to_string(),
            timeout: Duration::from_secs(60),
            extra_headers: HashMap::new(),
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

    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra_headers.insert(key.into(), value.into());
        self
    }
}

impl Site for ViduSite {
    fn id(&self) -> &str {
        "vidu"
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn build_url(&self, context: &UrlContext) -> String {
        // Vidu 的主调用点不走 OpenAI/Gemini 等协议端点。
        // 这里给一个不会被用到的占位路径；如果被误用，会尽快返回 404。
        let path = match context.action {
            Action::Generate => "/__vidu__/generate",
            Action::Stream => "/__vidu__/stream",
            Action::Embed => "/__vidu__/embed",
            Action::ImageGenerate => "/__vidu__/image",
        };
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }

    fn extra_headers(&self) -> HashMap<&str, &str> {
        self.extra_headers
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect()
    }

    fn timeout(&self) -> Duration {
        self.timeout
    }
}
