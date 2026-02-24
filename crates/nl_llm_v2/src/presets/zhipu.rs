use std::sync::Arc;
use crate::client::ClientBuilder;
use crate::site::base::openai::OpenAiSite;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::model::ZhipuModelResolver;
use crate::provider::zhipu::ZhipuExtension;

/// 智谱 AI (Zhipu) API 预设
///
/// 使用智谱 AI API，走 Bearer Token 认证。
/// 协议：OpenAI 兼容格式；模型默认：glm-4
///
/// ```
/// let client = LlmClient::from_preset("zhipu")
///     .with_api_key("xxx")
///     .build();
/// ```
pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url("https://open.bigmodel.cn/api/paas/v4"))
        .protocol(OpenAiProtocol {})
        .model_resolver(ZhipuModelResolver::new())
        .with_extension(Arc::new(ZhipuExtension::new()))
        .default_model("glm-4")
}
