use crate::client::ClientBuilder;
use crate::model::mimo::MiMoModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::mimo::MiMoExtension;
use crate::site::base::openai::OpenAiSite;
use std::sync::Arc;

/// Xiaomi MiMo 预设
///
/// # 平台特性
///
/// - **网关**: `https://api.xiaomimimo.com/v1`
/// - **认证**: 支持 `api-key` 和 `Bearer` 两种格式
/// - **协议**: OpenAI 兼容
///
/// # 支持的模型
///
/// | 模型 ID | 上下文 | 能力 | 说明 |
/// |---------|--------|------|------|
/// | `mimo-v2-flash` | 128K | CHAT, TOOLS, STREAMING, THINKING | 旗舰模型，支持思考模式 |
///
/// # 模型别名
///
/// | 别名 | 解析为 |
/// |------|--------|
/// | `mimo` | `mimo-v2-flash` |
/// | `flash` | `mimo-v2-flash` |
///
/// # 使用示例
///
/// ```rust
/// use nl_llm_v2::LlmClient;
///
/// let client = LlmClient::from_preset("mimo")
///     .expect("Preset should exist")
///     .with_api_key("your-api-key")
///     .build();
/// ```
const MIMO_BASE_URL: &str = "https://api.xiaomimimo.com/v1";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(MIMO_BASE_URL))
        .protocol(OpenAiProtocol {})
        .model_resolver(MiMoModelResolver::new())
        .with_extension(Arc::new(MiMoExtension::new().with_base_url(MIMO_BASE_URL)))
        .default_model("mimo-v2-flash")
}
