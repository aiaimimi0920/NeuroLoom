use crate::site::context::{Action, UrlContext};
use crate::site::traits::Site;
use std::collections::HashMap;
use std::time::Duration;

/// Anthropic 官方 API 站点
/// 特殊：使用 x-api-key header（而非 Authorization: Bearer）
///       需要 anthropic-version header
pub struct AnthropicSite {
    base_url: String,
    timeout: Duration,
    api_version: String,
}

impl AnthropicSite {
    pub fn new() -> Self {
        Self {
            base_url: "https://api.anthropic.com/v1".to_string(),
            timeout: Duration::from_secs(120),
            api_version: "2023-06-01".to_string(),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    pub fn with_api_version(mut self, version: impl Into<String>) -> Self {
        self.api_version = version.into();
        self
    }
}

impl Site for AnthropicSite {
    fn id(&self) -> &str {
        "anthropic"
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn build_url(&self, ctx: &UrlContext) -> String {
        // Anthropic 的端点结构：{base_url}/messages
        let endpoint = match ctx.action {
            Action::Generate | Action::Stream => "messages",
            Action::Embed => "messages", // fallback
            Action::ImageGenerate => "messages",
        };
        format!("{}/{}", self.base_url, endpoint)
    }

    /// Anthropic 需要特殊 headers:
    /// - anthropic-version: API 版本
    /// - content-type: application/json
    /// 注意：x-api-key 由 AnthropicApiKeyAuth 注入
    fn extra_headers(&self) -> HashMap<&str, &str> {
        let mut headers = HashMap::new();
        headers.insert("content-type", "application/json");
        // anthropic-version 需要动态引用，这里用 leak 保证 'static 生命周期
        // 在实际使用中这个值是固定的，只创建一次
        headers.insert("anthropic-version", self.api_version.as_str());
        headers
    }

    fn timeout(&self) -> Duration {
        self.timeout
    }
}
