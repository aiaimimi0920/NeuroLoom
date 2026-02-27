use crate::client::ClientBuilder;
use crate::model::zai::ZaiModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::zai::ZaiExtension;
use crate::site::base::openai::OpenAiSite;
use std::sync::Arc;

/// Z.AI（智谱 GLM 海外版）API 预设
///
/// Z.AI 是智谱 AI 的海外服务，使用 OpenAI 兼容协议。
///
/// ## 基本信息
///
/// - 官网：https://z.ai
/// - API 端点：`https://api.z.ai/api/paas/v4`
/// - 认证方式：Bearer Token
///
/// ## 基本用法
///
/// ```
/// let client = LlmClient::from_preset("zai")
///     .expect("Preset should exist")
///     .with_api_key("your-api-key")
///     .build();
/// ```
///
/// ## 使用别名
///
/// ```
/// let client = LlmClient::from_preset("zai")
///     .expect("Preset should exist")
///     .with_api_key("your-api-key")
///     .build();
///
/// // 使用便捷别名
/// let req = PrimitiveRequest::single_user_message("Hello")
///     .with_model("glm");  // 解析为 glm-5
/// ```
///
/// ## 支持的模型
///
/// | 模型 | 能力 | 上下文 |
/// |------|------|--------|
/// | glm-5 | Chat, Tools, Streaming | 128K |
/// | glm-4 | Chat, Vision, Tools, Streaming | 128K |
/// | glm-4-flash | Chat, Tools, Streaming | 128K |
/// | glm-4v | Chat, Vision, Streaming | 128K |
const ZAI_BASE_URL: &str = "https://api.z.ai/api/paas/v4";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(ZAI_BASE_URL))
        .protocol(OpenAiProtocol {})
        .model_resolver(ZaiModelResolver::new())
        .with_extension(Arc::new(ZaiExtension::new().with_base_url(ZAI_BASE_URL)))
        .default_model("glm-5")
}
