use std::sync::Arc;
use crate::client::ClientBuilder;
use crate::site::base::openai::OpenAiSite;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::model::DeepSeekModelResolver;
use crate::provider::deepseek::DeepSeekExtension;

/// DeepSeek API 预设
///
/// 使用 DeepSeek API，走 Bearer Token 认证。
/// 协议：OpenAI 兼容格式；模型默认：deepseek-chat
///
/// ```
/// let client = LlmClient::from_preset("deepseek")
///     .with_api_key("sk-xxx")
///     .build();
/// ```
pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url("https://api.deepseek.com/v1"))
        .protocol(OpenAiProtocol {})
        .model_resolver(DeepSeekModelResolver::new())
        .with_extension(Arc::new(DeepSeekExtension::new()))
        .default_model("deepseek-chat")
}
