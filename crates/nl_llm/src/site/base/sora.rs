use crate::site::context::UrlContext;
use crate::site::traits::Site;
use std::time::Duration;

pub struct SoraSite {
    timeout: Duration,
    base_url: String,
}

impl SoraSite {
    pub fn new() -> Self {
        Self {
            timeout: Duration::from_secs(60),
            base_url: "https://api.openai.com".to_string(),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
}

impl Site for SoraSite {
    fn id(&self) -> &str {
        "sora"
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn build_url(&self, _context: &UrlContext) -> String {
        format!("{}/v1/videos", self.base_url.trim_end_matches('/'))
    }

    fn timeout(&self) -> Duration {
        self.timeout
    }

    fn extra_headers(&self) -> std::collections::HashMap<&str, &str> {
        std::collections::HashMap::new()
    }
}
