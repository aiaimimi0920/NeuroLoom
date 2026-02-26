use crate::site::context::{Action, UrlContext};
use crate::site::traits::Site;
use std::collections::HashMap;
use std::time::Duration;

/// CloudCode (GeminiCLI / Antigravity) API 网关
pub struct CloudCodeSite {
    base_url: String,
    timeout: Duration,
    /// [新增] 允许自定义 User-Agent
    user_agent: String,
}

impl CloudCodeSite {
    pub fn new() -> Self {
        Self {
            base_url: "https://cloudcode-pa.googleapis.com/v1internal".to_string(),
            timeout: Duration::from_secs(180), // 代码编写经常等待时间过长需要放大限制
            user_agent: "google-api-nodejs-client/9.15.1".to_string(),
        }
    }

    /// [新增] 设置超时时间
    /// 原因：不同场景可能需要不同的超时设置
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// [新增] 设置自定义 Base URL
    /// 原因：支持私有部署或代理场景
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// [新增] 设置自定义 User-Agent
    /// 原因：某些场景可能需要伪装不同的客户端
    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = user_agent.into();
        self
    }
}

impl Default for CloudCodeSite {
    fn default() -> Self {
        Self::new()
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
        headers.insert("User-Agent", self.user_agent.as_str());
        headers.insert("X-Goog-Api-Client", "gl-python/3.12.0");
        headers.insert("Client-Metadata", r#"{"ideType":"IDE_UNSPECIFIED","platform":"PLATFORM_UNSPECIFIED","pluginType":"GEMINI"}"#);
        headers
    }

    fn timeout(&self) -> Duration {
        self.timeout
    }
}
