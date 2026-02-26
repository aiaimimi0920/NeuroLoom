use std::time::Duration;

use crate::site::context::{Action, UrlContext};
use crate::site::Site;

/// FastGPT API 站点定义
pub struct FastGptSite {}

impl Default for FastGptSite {
    fn default() -> Self {
        Self::new()
    }
}

impl FastGptSite {
    pub fn new() -> Self {
        Self {}
    }
}

impl Site for FastGptSite {
    fn id(&self) -> &str {
        "fastgpt"
    }

    fn base_url(&self) -> &str {
        "https://api.fastgpt.in/api/v1"
    }

    fn build_url(&self, ctx: &UrlContext) -> String {
        let base = self.base_url();
        match ctx.action {
             Action::Generate | Action::Stream => {
                format!("{}/chat/completions", base)
            }
            _ => base.to_string(),
        }
    }

    fn extra_headers(&self) -> std::collections::HashMap<&str, &str> {
        std::collections::HashMap::new()
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(60)
    }
}
