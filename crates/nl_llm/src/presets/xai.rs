use crate::client::ClientBuilder;
use crate::model::XaiModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::openai::OpenAiExtension;
use crate::site::base::openai::OpenAiSite;
use std::sync::Arc;

/// x.AI (Grok) API 预设
///
/// 使用官方 x.AI API，走 Bearer Token 认证。
/// 协议：OpenAI 格式；模型默认：grok-4-latest
///
/// ```rust,no_run
/// use nl_llm::LlmClient;
/// let client = LlmClient::from_preset("xai")
///     .expect("Preset should exist")
///     .with_api_key("xai-xxx")
///     .build();
/// ```
pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url("https://api.x.ai/v1"))
        .protocol(OpenAiProtocol {})
        .model_resolver(XaiModelResolver::new())
        .with_extension(Arc::new(OpenAiExtension::new()))
        .default_model("grok-4-latest")
}
