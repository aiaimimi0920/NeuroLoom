use std::collections::HashMap;
use std::time::Duration;

use crate::site::context::{Action, UrlContext};
use crate::site::traits::Site;

/// Vidu 官方 API 站点
///
/// Base URL: `https://api.vidu.cn`
///
/// 注意：Vidu 的核心能力是视频异步任务（img2video/start-end2video/reference2video 等）。
/// 这些能力通过 `ProviderExtension::{submit_video_task, fetch_video_task}` 直接调用 Vidu 的任务 API。
///
/// 这里的 `build_url` 仅用于保持 Pipeline 的完整性（当用户误用 complete/stream 时会走到这里）。
pub struct ViduSite {
    base_url: String,
    timeout: Duration,
    extra_headers: HashMap<String, String>,
}

impl ViduSite {
    pub fn new() -> Self {
        Self {
            base_url: "https://api.vidu.cn".to_string(),
            timeout: Duration::from_secs(60),
            extra_headers: HashMap::new(),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra_headers.insert(key.into(), value.into());
        self
    }
}

impl Site for ViduSite {
    fn id(&self) -> &str {
        "vidu_base"
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn build_url(&self, context: &UrlContext) -> String {
        // Vidu 并非 OpenAI Chat 接口；此处仅提供一个“占位”路径。
        // 视频任务 API 不走此入口。
        let path = match context.action {
            Action::Generate => "/v1/chat/completions",
            Action::Stream => "/v1/chat/completions",
            Action::Embed => "/v1/embeddings",
            Action::ImageGenerate => "/v1/images/generations",
        };

        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }

    fn extra_headers(&self) -> HashMap<&str, &str> {
        self.extra_headers
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect()
    }

    fn timeout(&self) -> Duration {
        self.timeout
    }
}
