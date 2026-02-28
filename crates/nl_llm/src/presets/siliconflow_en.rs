use crate::client::ClientBuilder;
use crate::model::siliconflow::SiliconFlowModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::siliconflow::SiliconFlowExtension;
use crate::site::base::openai::OpenAiSite;
use std::sync::Arc;

/// SiliconFlow EN (国际版) 预设
///
/// # 平台特性
///
/// - **网关**: `https://api.siliconflow.com/v1`（注意 .com 而非 .cn）
/// - **认证**: 标准 Bearer 格式
/// - **协议**: OpenAI 兼容
///
/// # 与中国版的区别
///
/// | 属性 | 中国版 (siliconflow) | 国际版 (siliconflow_en) |
/// |------|---------------------|----------------------|
/// | 域名 | api.siliconflow.cn | api.siliconflow.com |
/// | 默认模型 | Pro/moonshotai/Kimi-K2.5 | deepseek-ai/DeepSeek-V3 |
/// | 可用模型 | 更多国产模型 | 以国际通用模型为主 |
///
/// # 使用示例
///
/// ```rust
/// use nl_llm::LlmClient;
///
/// let client = LlmClient::from_preset("siliconflow_en")
///     .expect("Preset should exist")
///     .with_api_key("your-api-key")
///     .build();
/// ```
const SILICONFLOW_EN_BASE_URL: &str = "https://api.siliconflow.com/v1";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(SILICONFLOW_EN_BASE_URL))
        .protocol(OpenAiProtocol {})
        .model_resolver(SiliconFlowModelResolver::new())
        .with_extension(Arc::new(
            SiliconFlowExtension::new().with_base_url(SILICONFLOW_EN_BASE_URL),
        ))
        .default_model("deepseek-ai/DeepSeek-V3")
}
