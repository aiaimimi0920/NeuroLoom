use std::collections::HashMap;
use std::time::Duration;

use crate::site::context::{Action, UrlContext};
use crate::site::traits::Site;

/// Amazon Bedrock 站点配置
///
/// AWS Bedrock 使用特殊的 URL 结构来调用模型：
/// `https://bedrock-runtime.{region}.amazonaws.com/model/{model-id}/converse`
///
/// ## 认证模式
///
/// ### AK/SK 模式（AWS SigV4 签名）
/// - 需要 `AWS_ACCESS_KEY_ID` + `AWS_SECRET_ACCESS_KEY`
/// - 每个请求需要计算 HMAC-SHA256 签名
///
/// ### API Key 模式
/// - 使用简单的 API Key
/// - 通过 `Authorization: Bearer <key>` 或自定义 header
///
/// ## URL 格式
///
/// Bedrock 原生 API:
/// `https://bedrock-runtime.{region}.amazonaws.com/model/{model-id}/converse`
///
/// OpenAI 兼容层 (如果可用):
/// `https://bedrock-runtime.{region}.amazonaws.com/v1/chat/completions`
pub struct BedrockSite {
    /// AWS 区域 (如 us-east-1)
    region: String,
    /// 是否使用 OpenAI 兼容模式
    openai_compat: bool,
    timeout: Duration,
    extra_headers: HashMap<String, String>,
}

impl BedrockSite {
    pub fn new(region: impl Into<String>) -> Self {
        Self {
            region: region.into(),
            openai_compat: true, // 默认使用 OpenAI 兼容模式
            timeout: Duration::from_secs(120),
            extra_headers: HashMap::new(),
        }
    }

    /// 使用 Bedrock 原生 Converse API
    pub fn with_native_api(mut self) -> Self {
        self.openai_compat = false;
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

    fn endpoint(&self) -> String {
        format!("https://bedrock-runtime.{}.amazonaws.com", self.region)
    }
}

impl Site for BedrockSite {
    fn id(&self) -> &str {
        "aws_bedrock"
    }

    fn base_url(&self) -> &str {
        // 返回格式化的 endpoint（注意：这里会每次创建新 String）
        // 实际使用 build_url 中的完整 URL
        "https://bedrock-runtime.us-east-1.amazonaws.com"
    }

    fn build_url(&self, context: &UrlContext) -> String {
        let endpoint = self.endpoint();

        if self.openai_compat {
            // OpenAI 兼容模式
            let path = match context.action {
                Action::Generate | Action::Stream => "/v1/chat/completions",
                Action::Embed => "/v1/embeddings",
                Action::ImageGenerate => "/v1/images/generations",
            };
            format!("{}{}", endpoint, path)
        } else {
            // Bedrock 原生 Converse API
            let action = match context.action {
                Action::Generate => "converse",
                Action::Stream => "converse-stream",
                Action::Embed => "invoke",
                Action::ImageGenerate => "invoke",
            };
            format!("{}/model/{}/{}", endpoint, context.model, action)
        }
    }

    fn timeout(&self) -> Duration {
        self.timeout
    }

    fn extra_headers(&self) -> HashMap<&str, &str> {
        self.extra_headers
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect()
    }
}
