use std::time::Duration;
use crate::site::Site;

/// 腾讯混元大模型站点
#[derive(Debug, Clone)]
pub struct HunyuanSite {
    base_url: String,
}

impl Default for HunyuanSite {
    fn default() -> Self {
        Self::new()
    }
}

impl HunyuanSite {
    pub fn new() -> Self {
        Self {
            base_url: "https://api.hunyuan.cloud.tencent.com/v1".to_string(),
        }
    }
}

impl Site for HunyuanSite {
    fn id(&self) -> &str {
        "hunyuan"
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn build_url(&self, ctx: &crate::site::context::UrlContext) -> String {
        match ctx.action {
            crate::site::context::Action::Generate | crate::site::context::Action::Stream => {
                format!("{}/chat/completions", self.base_url)
            }
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
