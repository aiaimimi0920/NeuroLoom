use std::sync::Arc;
use crate::client::ClientBuilder;
use crate::site::base::openai::OpenAiSite;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::model::OpenAiModelResolver;
use crate::provider::openai::OpenAiExtension;

/// OpenAI API 预设
///
/// 使用官方 OpenAI API，走 Bearer Token 认证。
/// 协议：OpenAI 格式；模型默认：gpt-4o
///
/// ```
/// let client = LlmClient::from_preset("openai")
///     .with_api_key("sk-xxx")
///     .build();
/// ```
pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new())
        .protocol(OpenAiProtocol {})
        .model_resolver(OpenAiModelResolver::new())
        .with_extension(Arc::new(OpenAiExtension::new()))
        .default_model("gpt-4o")
}
