use std::collections::HashMap;
use std::time::Duration;

use crate::site::context::{Action, UrlContext};
use crate::site::traits::Site;

/// Azure OpenAI 站点配置
///
/// Azure OpenAI 使用与标准 OpenAI 不同的 URL 结构：
/// `{endpoint}/openai/deployments/{deployment}/chat/completions?api-version={version}`
///
/// ## URL 格式
///
/// - `model` 字段作为 deployment name 使用
/// - 必须携带 `api-version` 查询参数
/// - 认证使用 `api-key` 请求头（非 `Authorization: Bearer`）
pub struct AzureOpenAiSite {
    /// Azure 资源端点（如 `https://myresource.openai.azure.com`）
    endpoint: String,
    /// API 版本（如 `2024-12-01-preview`）
    api_version: String,
    timeout: Duration,
    extra_headers: HashMap<String, String>,
}

impl AzureOpenAiSite {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into().trim_end_matches('/').to_string(),
            api_version: "2024-12-01-preview".to_string(),
            timeout: Duration::from_secs(120),
            extra_headers: HashMap::new(),
        }
    }

    pub fn with_api_version(mut self, version: impl Into<String>) -> Self {
        self.api_version = version.into();
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

impl Site for AzureOpenAiSite {
    fn id(&self) -> &str {
        "azure_openai"
    }

    fn base_url(&self) -> &str {
        &self.endpoint
    }

    /// Azure OpenAI URL 格式:
    /// `{endpoint}/openai/deployments/{deployment}/chat/completions?api-version={version}`
    ///
    /// `context.model` 作为 deployment name 使用
    fn build_url(&self, context: &UrlContext) -> String {
        let path = match context.action {
            Action::Generate | Action::Stream => "chat/completions",
            Action::Embed => "embeddings",
            Action::ImageGenerate => "images/generations",
        };

        format!(
            "{}/openai/deployments/{}/{}?api-version={}",
            self.endpoint, context.model, path, self.api_version
        )
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
