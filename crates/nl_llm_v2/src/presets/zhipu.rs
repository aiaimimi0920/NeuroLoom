use crate::client::ClientBuilder;
use crate::model::ZhipuModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::zhipu::ZhipuExtension;
use crate::site::base::openai::OpenAiSite;
use std::sync::Arc;

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
/// 智谱 API 基础 URL
const ZHIPU_BASE_URL: &str = "https://open.bigmodel.cn/api/paas/v4";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(ZHIPU_BASE_URL))
        .protocol(OpenAiProtocol {})
        .model_resolver(ZhipuModelResolver::new())
        .with_extension(Arc::new(
            ZhipuExtension::new().with_base_url(ZHIPU_BASE_URL),
        ))
        .default_model("glm-5")
}
