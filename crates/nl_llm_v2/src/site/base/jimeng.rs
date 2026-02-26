use crate::site::context::UrlContext;
use crate::site::traits::Site;
use std::collections::HashMap;
use std::time::Duration;

pub struct JimengSite {
    base_url: String,
}

impl JimengSite {
    pub fn new() -> Self {
        Self {
            base_url: "https://visual.volcengineapi.com".to_string(),
        }
    }
}

impl Site for JimengSite {
    fn id(&self) -> &str {
        "jimeng"
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn build_url(&self, _ctx: &UrlContext) -> String {
        self.base_url.clone()
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(60)
    }

    fn extra_headers(&self) -> HashMap<&str, &str> {
        HashMap::new()
    }
}
