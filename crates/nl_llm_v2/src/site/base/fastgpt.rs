use std::collections::HashMap;
use std::time::Duration;

use crate::site::context::{Action, UrlContext};
use crate::site::Site;

/// FastGPT API 站点定义
pub struct FastGptSite {
    base_url: String,
    timeout: Duration,
    extra_headers: HashMap<String, String>,
}

impl Default for FastGptSite {
    fn default() -> Self {
        Self::new()
    }
}

impl FastGptSite {
    pub fn new() -> Self {
        Self {
            base_url: "https://api.fastgpt.in/api/v1".to_string(),
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

impl Site for FastGptSite {
    fn id(&self) -> &str {
        "fastgpt"
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn build_url(&self, ctx: &UrlContext) -> String {
        let base = self.base_url.trim_end_matches('/');
        match ctx.action {
            Action::Generate | Action::Stream => {
                format!("{}/chat/completions", base)
            }
            Action::Embed => format!("{}/embeddings", base),
            Action::ImageGenerate => format!("{}/images/generations", base),
        }
    }

    fn extra_headers(&self) -> std::collections::HashMap<&str, &str> {
        self.extra_headers
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect()
    }

    fn timeout(&self) -> Duration {
        self.timeout
    }
}
