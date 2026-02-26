use std::time::Duration;
use crate::site::traits::Site;
use crate::site::context::UrlContext;

pub struct ReplicateSite {
    timeout: Duration,
    base_url: String,
}

impl ReplicateSite {
    pub fn new() -> Self {
        Self {
            timeout: Duration::from_secs(60),
            base_url: "https://api.replicate.com".to_string(),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
}

impl Site for ReplicateSite {
    fn id(&self) -> &str {
        "replicate_video"
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn build_url(&self, _context: &UrlContext) -> String {
        format!("{}/v1/predictions", self.base_url.trim_end_matches('/'))
    }

    fn timeout(&self) -> Duration {
        self.timeout
    }

    fn extra_headers(&self) -> std::collections::HashMap<&str, &str> {
        let mut map = std::collections::HashMap::new();
        map.insert("Prefer", "wait");
        map
    }
}
