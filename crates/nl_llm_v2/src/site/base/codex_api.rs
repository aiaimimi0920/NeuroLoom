use crate::site::context::UrlContext;
use crate::site::traits::Site;
use std::collections::HashMap;
use std::time::Duration;

/// OpenAI Codex API 站点（API Key 模式）
///
/// 通过 OpenAI 官方 API 访问 Codex 模型。
/// Base URL: `https://api.openai.com/v1`
/// 端点: `/responses`
pub struct CodexApiSite {
    base_url: String,
    timeout: Duration,
}

impl CodexApiSite {
    pub fn new() -> Self {
        Self {
            base_url: "https://api.openai.com/v1".to_string(),
            timeout: Duration::from_secs(120),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
}

impl Site for CodexApiSite {
    fn id(&self) -> &str {
        "codex_api"
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn build_url(&self, _ctx: &UrlContext) -> String {
        // Codex API 所有操作都走 /responses 端点
        format!("{}/responses", self.base_url)
    }

    fn extra_headers(&self) -> HashMap<&str, &str> {
        // API Key 模式无需额外 headers
        // Content-Type 由 SendStage .json() 自动设置
        HashMap::new()
    }

    fn timeout(&self) -> Duration {
        self.timeout
    }
}
