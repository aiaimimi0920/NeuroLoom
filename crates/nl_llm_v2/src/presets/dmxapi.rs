use crate::client::ClientBuilder;
use crate::model::dmxapi::DmxApiModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::dmxapi::DmxApiExtension;
use crate::site::base::openai::OpenAiSite;
use std::sync::Arc;

/// DMXAPI 聚合平台预设
///
/// # 平台特性
///
/// - **网关**: `https://www.dmxapi.cn/v1`（注意需要 www 前缀）
/// - **认证**: 标准 Bearer 格式
/// - **协议**: OpenAI 兼容
/// - **类型**: API 聚合平台
///
/// # 支持的模型
///
/// | 模型 ID | 上下文 | 说明 |
/// |---------|--------|------|
/// | `claude-sonnet-4-5-20250929` | 200K | Claude Sonnet 4.5 |
/// | `claude-opus-4-6` | 200K | Claude Opus 4.6 |
/// | `gpt-4o` | 128K | GPT-4o |
/// | `gpt-4.1` | 1M | GPT-4.1 |
///
/// # 使用示例
///
/// ```rust
/// use nl_llm_v2::LlmClient;
///
/// let client = LlmClient::from_preset("dmxapi")
///     .expect("Preset should exist")
///     .with_api_key("your-api-key")
///     .build();
/// ```
const DMXAPI_BASE_URL: &str = "https://www.dmxapi.cn/v1";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(DMXAPI_BASE_URL))
        .protocol(OpenAiProtocol {})
        .model_resolver(DmxApiModelResolver::new())
        .with_extension(Arc::new(DmxApiExtension::new().with_base_url(DMXAPI_BASE_URL)))
        .default_model("gpt-4o-mini")
}
