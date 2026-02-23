use std::collections::HashMap;
use std::time::Duration;
use crate::site::traits::Site;
use crate::site::context::{UrlContext, Action};

/// CloudCode (GeminiCLI / Antigravity) API 网关
pub struct CloudCodeSite {
    base_url: String,
    timeout: Duration,
}

impl CloudCodeSite {
    pub fn new() -> Self {
        Self {
            base_url: "https://cloudcode-pa.googleapis.com/v1internal".to_string(),
            timeout: Duration::from_secs(180), // 代码编写经常等待时间过长需要放大限制
        }
    }
}

impl Site for CloudCodeSite {
    fn id(&self) -> &str {
        "cloudcode"
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn build_url(&self, ctx: &UrlContext) -> String {
        let action = match ctx.action {
            Action::Generate => "generateContent",
            Action::Stream => "streamGenerateContent?alt=sse",
            Action::Embed => "generateContent", // default fallback for cloudcode
            Action::ImageGenerate => "generateContent",
        };
        format!("{}:{}", self.base_url, action)
    }

    // [修复] 返回 CloudCode 所需的额外 Headers
    // 原因：CloudCode API 需要特定的 Headers，之前在 before_send hook 中设置不可靠
    fn extra_headers(&self) -> HashMap<&str, &str> {
        let mut headers = HashMap::new();
        headers.insert("Content-Type", "application/json");
        headers.insert("User-Agent", "google-api-nodejs-client/9.15.1");
        headers.insert("X-Goog-Api-Client", "gl-python/3.12.0");
        headers.insert("Client-Metadata", r#"{"ideType":"IDE_UNSPECIFIED","platform":"PLATFORM_UNSPECIFIED","pluginType":"GEMINI"}"#);
        headers
    }

    fn timeout(&self) -> Duration {
        self.timeout
    }
}
