use crate::site::context::{Action, UrlContext};
use crate::site::Site;
use std::time::Duration;

/// Coze API 站点定义
pub struct CozeSite {
    id: String,
    base_url: String,
    extra_headers: std::collections::HashMap<String, String>,
    timeout: Duration,
}

impl Default for CozeSite {
    fn default() -> Self {
        Self::new()
    }
}

impl CozeSite {
    pub fn new() -> Self {
        Self {
            id: "coze".to_string(),
            base_url: "https://api.coze.com/v3".to_string(),
            extra_headers: std::collections::HashMap::new(),
            timeout: Duration::from_secs(60),
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

impl Site for CozeSite {
    fn id(&self) -> &str {
        &self.id
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn build_url(&self, ctx: &UrlContext) -> String {
        match ctx.action {
            Action::Generate | Action::Stream => format!("{}/chat", self.base_url),
            _ => format!("{}/chat", self.base_url),
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
