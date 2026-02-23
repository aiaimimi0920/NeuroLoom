use std::collections::HashMap;
use std::time::Duration;

use crate::site::traits::Site;
use crate::site::context::{UrlContext, Action};

/// OpenAI 平台默认网关配置
pub struct OpenAiSite {
    base_url: String,
    timeout: Duration,
    // [修复] 使用 HashMap 存储额外 headers，而非 HeaderMap
    // 原因：extra_headers 接口返回 HashMap<&str, &str>，需要兼容
    extra_headers: HashMap<String, String>,
}

impl OpenAiSite {
    pub fn new() -> Self {
        Self {
            base_url: "https://api.openai.com/v1".to_string(),
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

impl Site for OpenAiSite {
    fn id(&self) -> &str {
        "openai_base"
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

        // 简单的 URL 拼接
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }

    fn timeout(&self) -> Duration {
        self.timeout
    }

    // [修复] 正确返回 extra_headers
    // 原因：之前返回空 HashMap，导致 with_header 设置的 headers 丢失
    fn extra_headers(&self) -> HashMap<&str, &str> {
        self.extra_headers.iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect()
    }
}
