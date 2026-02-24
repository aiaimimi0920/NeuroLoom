use std::sync::Arc;
use crate::client::ClientBuilder;
use crate::site::base::openai::OpenAiSite;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::model::MoonshotModelResolver;
use crate::provider::moonshot::MoonshotExtension;

/// Moonshot (月之暗面) API 预设
///
/// 使用 Moonshot API，走 Bearer Token 认证。
/// 协议：OpenAI 兼容格式；模型默认：moonshot-v1-8k
///
/// ```
/// let client = LlmClient::from_preset("moonshot")
///     .with_api_key("sk-xxx")
///     .build();
/// ```
pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url("https://api.moonshot.cn/v1"))
        .protocol(OpenAiProtocol {})
        .model_resolver(MoonshotModelResolver::new())
        .with_extension(Arc::new(MoonshotExtension::new()))
        .default_model("moonshot-v1-8k")
}
