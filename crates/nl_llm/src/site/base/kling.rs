use std::collections::HashMap;
use std::time::Duration;

use crate::site::context::{Action, UrlContext};
use crate::site::traits::Site;

/// 可灵 AI (Kling) 服务网关
///
/// 官方 Base URL: `https://api.klingai.com`
pub struct KlingSite {
    base_url: String,
    timeout: Duration,
    extra_headers: HashMap<String, String>,
}

impl KlingSite {
    pub fn new() -> Self {
        Self {
            base_url: "https://api-beijing.klingai.com".to_string(),
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

impl Site for KlingSite {
    fn id(&self) -> &str {
        "kling_base"
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn build_url(&self, context: &UrlContext) -> String {
        // 由于可灵的重点是视频/图像生成，如果是基础接口则降级为 chat 兼容
        // 实际的视频任务调度会在 Extension 接口中单独发起请求
        let path = match context.action {
            Action::Generate => "/v1/chat/completions",
            Action::Stream => "/v1/chat/completions",
            Action::Embed => "/v1/embeddings",
            Action::ImageGenerate => "/v1/images/generations",
        };

        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }

    fn timeout(&self) -> Duration {
        self.timeout
    }

    fn extra_headers(&self) -> HashMap<&str, &str> {
        self.extra_headers
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect()
    }
}
