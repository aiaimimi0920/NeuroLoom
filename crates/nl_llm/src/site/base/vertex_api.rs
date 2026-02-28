use crate::site::context::{Action, UrlContext};
use crate::site::traits::Site;
use std::collections::HashMap;
use std::time::Duration;

/// Vertex AI (API Key 模式) 网关
///
/// 独立于 VertexSite (SA JSON 模式)。
///
/// 关键区别：
/// - SA JSON 模式走 `aiplatform.googleapis.com` (需要 Bearer Token)
/// - API Key 模式走 `generativelanguage.googleapis.com` (通过 URL `?key=xxx`)
///
/// 参考: vertex_config.txt
///   SA 模式: https://{region}-aiplatform.googleapis.com/v1/projects/{proj}/locations/{region}/publishers/google/models/{model}:generateContent
///   API Key 模式: https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={key}
pub struct VertexApiSite {
    api_key: String,
    timeout: Duration,
}

impl VertexApiSite {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            timeout: Duration::from_secs(120),
        }
    }

    /// 设置超时
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

impl Site for VertexApiSite {
    fn id(&self) -> &str {
        "vertex_api"
    }

    fn base_url(&self) -> &str {
        "https://generativelanguage.googleapis.com"
    }

    fn build_url(&self, ctx: &UrlContext) -> String {
        let action_suffix = match ctx.action {
            Action::Generate => "generateContent",
            Action::Stream => "streamGenerateContent?alt=sse",
            _ => "generateContent",
        };

        let mut url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:{}",
            ctx.model, action_suffix
        );

        // API Key 追加到 URL query
        if !self.api_key.is_empty() {
            let sep = if url.contains('?') { "&" } else { "?" };
            url.push_str(&format!("{}key={}", sep, self.api_key));
        }

        url
    }

    fn extra_headers(&self) -> HashMap<&str, &str> {
        HashMap::new()
    }

    fn timeout(&self) -> Duration {
        self.timeout
    }
}
