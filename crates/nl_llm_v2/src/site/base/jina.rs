use std::time::Duration;

use crate::site::context::{Action, UrlContext};
use crate::site::Site;

/// Jina 官网地址与端点路由实现
pub struct JinaSite {}

impl Default for JinaSite {
    fn default() -> Self {
        Self::new()
    }
}

impl JinaSite {
    pub fn new() -> Self {
        Self {}
    }
}

impl Site for JinaSite {
    fn id(&self) -> &str {
        "jina"
    }

    fn base_url(&self) -> &str {
        "https://api.jina.ai/v1"
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
