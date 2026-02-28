use std::time::Duration;

use crate::site::context::{Action, UrlContext};
use crate::site::Site;

/// Mistral API 站点定义
pub struct MistralSite {}

impl Default for MistralSite {
    fn default() -> Self {
        Self::new()
    }
}

impl MistralSite {
    pub fn new() -> Self {
        Self {}
    }
}

impl Site for MistralSite {
    fn id(&self) -> &str {
        "mistral"
    }

    fn base_url(&self) -> &str {
        "https://api.mistral.ai/v1"
    }

    fn build_url(&self, ctx: &UrlContext) -> String {
        let base = self.base_url();
        match ctx.action {
            Action::Generate | Action::Stream => {
                format!("{}/chat/completions", base)
            }
            Action::Embed => {
                format!("{}/embeddings", base)
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
