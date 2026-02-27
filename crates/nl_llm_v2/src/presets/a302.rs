use crate::client::ClientBuilder;
use crate::model::OpenAiModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::openai::OpenAiExtension;
use crate::site::base::openai::OpenAiSite;
use std::sync::Arc;

/// 302.ai API 预设
///
/// 使用 302.ai API，走 Bearer Token 认证。
/// 协议：OpenAI 格式；模型默认：gpt-4o
/// Base URL: https://api.302.ai/v1
///
/// ```
/// let client = LlmClient::from_preset("302.ai")
///     .with_api_key("sk-xxx")
///     .build();
/// ```
pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url("https://api.302.ai/v1"))
        .protocol(OpenAiProtocol {})
        .model_resolver(OpenAiModelResolver::new())
        .with_extension(Arc::new(OpenAiExtension::new()))
        .default_model("gpt-3.5-turbo")
}
