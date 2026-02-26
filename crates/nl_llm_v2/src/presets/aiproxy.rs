use crate::client::ClientBuilder;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::aiproxy::AiProxyModelResolver;
use crate::provider::openai::OpenAiExtension;
use crate::site::base::openai::OpenAiSite;

use std::sync::Arc;

/// AI Proxy API 预设
///
/// 模拟并映射 new-api Type 10 / 21 的机制，通过 https://api.aiproxy.io 访问。
/// 完全兼容 OpenAI 的请求载荷，并提供模型列表的扩展功能（OpenAiExtension）。
pub fn builder() -> ClientBuilder {
    // 默认使用官方基础地址，允许用户通过环境变量覆盖
    let base_url = std::env::var("AIPROXY_BASE_URL")
        .unwrap_or_else(|_| "https://api.aiproxy.io/v1".to_string());

    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(&base_url))
        .protocol(OpenAiProtocol {})
        .model_resolver(AiProxyModelResolver::new())
        .with_extension(Arc::new(OpenAiExtension::new()))
}
