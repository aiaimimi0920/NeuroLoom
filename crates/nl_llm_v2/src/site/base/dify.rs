use std::time::Duration;
use crate::site::Site;

/// Dify 大模型站点
/// 使用 Dify 的 Chat App API
#[derive(Debug, Clone)]
pub struct DifySite {
    base_url: String,
}

impl Default for DifySite {
    fn default() -> Self {
        Self::new()
    }
}

impl DifySite {
    pub fn new() -> Self {
        Self {
            base_url: "https://api.dify.ai/v1".to_string(),
        }
    }
}

impl Site for DifySite {
    fn id(&self) -> &str {
        "dify"
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn build_url(&self, ctx: &crate::site::context::UrlContext) -> String {
        match ctx.action {
            // Dify 的 Chat API Endpoint
            crate::site::context::Action::Generate | crate::site::context::Action::Stream => {
                format!("{}/chat-messages", self.base_url)
            }
            // Dify 并不支持 OpenAI 风格的 /models API
            _ => self.base_url.clone(),
        }
    }

    fn extra_headers(&self) -> std::collections::HashMap<&str, &str> {
        let mut headers = std::collections::HashMap::new();
        headers.insert("Content-Type", "application/json");
        headers
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(60)
    }
}
