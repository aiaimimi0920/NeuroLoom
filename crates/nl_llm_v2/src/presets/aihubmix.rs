use crate::client::ClientBuilder;
use crate::model::aihubmix::AiHubMixModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::aihubmix::AiHubMixExtension;
use crate::site::base::openai::OpenAiSite;
use std::sync::Arc;

/// AiHubMix 聚合平台预设
///
/// # 平台特性
///
/// - **网关**: `https://aihubmix.com/v1`
/// - **认证**: 标准 Bearer 格式
/// - **协议**: OpenAI 兼容
/// - **类型**: API 聚合平台，支持多种模型
///
/// # 免费模型
///
/// | 模型 ID | 上下文 | 说明 |
/// |---------|--------|------|
/// | `gpt-4o-free` | 1M | GPT-4o 免费版 |
/// | `gpt-4.1-free` | 1M | GPT-4.1 免费版 |
/// | `gemini-2.0-flash-free` | 1M | Gemini 2.0 Flash 免费版 |
/// | `gemini-3-flash-preview-free` | 1M | Gemini 3 Flash 预览免费版 |
///
/// # 使用示例
///
/// ```rust
/// use nl_llm_v2::LlmClient;
///
/// let client = LlmClient::from_preset("aihubmix")
///     .expect("Preset should exist")
///     .with_api_key("your-api-key")
///     .build();
/// ```
const AIHUBMIX_BASE_URL: &str = "https://aihubmix.com/v1";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(AIHUBMIX_BASE_URL))
        .protocol(OpenAiProtocol {})
        .model_resolver(AiHubMixModelResolver::new())
        .with_extension(Arc::new(
            AiHubMixExtension::new().with_base_url(AIHUBMIX_BASE_URL),
        ))
        .default_model("gpt-4o-free")
}
