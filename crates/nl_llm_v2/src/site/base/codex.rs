use std::collections::HashMap;
use std::time::Duration;
use crate::site::traits::Site;
use crate::site::context::{UrlContext, Action};

/// Codex CLI 版本（参考 CLIProxyAPI）
const CODEX_CLIENT_VERSION: &str = "0.101.0";

/// OpenAI Codex API 站点
///
/// Base URL: `https://chatgpt.com/backend-api/codex`
/// 端点: `/responses`（Generate 和 Stream 共用）
pub struct CodexSite {
    base_url: String,
    timeout: Duration,
}

impl CodexSite {
    pub fn new() -> Self {
        Self {
            base_url: "https://chatgpt.com/backend-api/codex".to_string(),
            timeout: Duration::from_secs(120),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
}

impl Site for CodexSite {
    fn id(&self) -> &str {
        "codex"
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn build_url(&self, ctx: &UrlContext) -> String {
        let endpoint = match ctx.action {
            Action::Generate | Action::Stream => "responses",
            Action::Embed => "responses",
            Action::ImageGenerate => "responses",
        };
        format!("{}/{}", self.base_url, endpoint)
    }

    fn extra_headers(&self) -> HashMap<&str, &str> {
        let mut headers = HashMap::new();
        // 注意：不设 content-type，因为 SendStage 的 .json() 已自动设置
        // 重复的 Content-Type 会导致 Codex 后端返回 "Unsupported content type"
        headers.insert("accept", "application/json");
        headers.insert("version", CODEX_CLIENT_VERSION);
        headers.insert("connection", "keep-alive");
        headers
    }

    fn timeout(&self) -> Duration {
        self.timeout
    }
}
