use std::time::Duration;
use crate::site::traits::Site;
use crate::site::context::UrlContext;

pub struct DoubaoSite {
    timeout: Duration,
}

impl DoubaoSite {
    pub fn new() -> Self {
        Self {
            timeout: Duration::from_secs(60),
        }
    }
}

impl Site for DoubaoSite {
    fn id(&self) -> &str {
        "doubao_video"
    }

    fn base_url(&self) -> &str {
        "https://ark.cn-beijing.volces.com"
    }

    fn build_url(&self, _context: &UrlContext) -> String {
        // Only accessed if used via protocol layout directly rather than extensions. 
        // For video, ProviderExtension takes priority.
        format!("{}/api/v3/contents/generations/tasks", self.base_url())
    }

    fn timeout(&self) -> Duration {
        self.timeout
    }

    fn extra_headers(&self) -> std::collections::HashMap<&str, &str> {
        std::collections::HashMap::new()
    }
}
