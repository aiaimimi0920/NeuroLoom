use std::time::Duration;

use crate::site::context::{Action, UrlContext};
use crate::site::traits::Site;

/// Gemini 官方原生端点站点
pub struct GeminiSite {
    base_url: String,
    timeout: Duration,
    // [移除] headers 字段
    // 原因：Gemini 使用 query params 传递 API key，不需要额外 headers
    api_key: String,
}

impl GeminiSite {
    pub fn new() -> Self {
        Self {
            base_url: "https://generativelanguage.googleapis.com/v1beta/models".to_string(),
            timeout: Duration::from_secs(60),
            api_key: String::new(),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = key.into();
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

impl Site for GeminiSite {
    fn id(&self) -> &str {
        "gemini_base"
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn build_url(&self, context: &UrlContext) -> String {
        // e.g. /models/gemini-2.5-pro:generateContent
        let model = if context.model.is_empty() {
            "gemini-1.5-pro-latest"
        } else {
            context.model
        };

        let path = match context.action {
            Action::Generate => format!(":generateContent"),
            Action::Stream => format!(":streamGenerateContent?alt=sse"),
            Action::Embed => format!(":embedContent"),
            _ => format!(":generateContent"),
        };

        let mut base = format!("{}/{}{}", self.base_url.trim_end_matches('/'), model, path);

        if !self.api_key.is_empty() {
            let sep = if base.contains('?') { "&" } else { "?" };
            base.push_str(&format!("{}key={}", sep, self.api_key));
        }

        base
    }

    fn timeout(&self) -> Duration {
        self.timeout
    }

    fn extra_headers(&self) -> std::collections::HashMap<&str, &str> {
        // Typically returns string references from static or self. Not fully used by pipeline standard if empty.
        std::collections::HashMap::new()
    }
}
