use std::collections::HashMap;
use std::time::Duration;

use crate::site::traits::Site;
use crate::site::context::{UrlContext, Action};

/// MokaAI 服务网关
/// 这是专门针对 https://api.moka.ai 配置的基础站点，其路口格式与 OpenAI 有所不同。
pub struct MokaSite {
    base_url: String,
    timeout: Duration,
    extra_headers: HashMap<String, String>,
}

impl MokaSite {
    pub fn new() -> Self {
        Self {
            base_url: "https://api.moka.ai".to_string(),
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

impl Site for MokaSite {
    fn id(&self) -> &str {
        "moka_base"
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn build_url(&self, context: &UrlContext) -> String {
        // MokaAI 具有专门定制的特殊端点路由
        let path = match context.action {
            // Moka 会直接将 Generate/Stream 解析到 /chat/ 结尾
            // 相关实现参考在 new-api/relay/channel/mokaai/adaptor.go 的逻辑
            Action::Generate => "/chat/",
            Action::Stream => "/chat/",
            Action::Embed => "/embeddings",
            Action::ImageGenerate => "/images/generations",
        };

        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }

    fn timeout(&self) -> Duration {
        self.timeout
    }

    fn extra_headers(&self) -> HashMap<&str, &str> {
        self.extra_headers.iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect()
    }
}
