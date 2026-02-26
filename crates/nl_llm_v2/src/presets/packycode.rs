use crate::client::ClientBuilder;
use crate::model::packycode::PackyCodeModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::packycode::PackyCodeExtension;
use crate::site::base::openai::OpenAiSite;
use std::sync::Arc;

/// PackyCode 聚合平台预设
///
/// # 平台特性
///
/// - **网关**: `https://api.packycode.com/v1`
/// - **认证**: 标准 Bearer 格式，API Key 以 `sk-` 为前缀
/// - **协议**: OpenAI 兼容
/// - **类型**: 国内 AI 模型 API 中转服务平台
///
/// # 支持的模型
///
/// | 模型 ID | 上下文 | 说明 |
/// |---------|--------|------|
/// | `gpt-4o` | 128K | GPT-4o，支持视觉 |
/// | `gpt-4o-mini` | 128K | GPT-4o Mini，支持视觉 |
/// | `gpt-4.1` | 1M | GPT-4.1，支持视觉 |
/// | `gpt-4.1-mini` | 1M | GPT-4.1 Mini，支持视觉 |
/// | `claude-sonnet-4-5-20250929` | 200K | Claude Sonnet 4.5，支持思考 |
/// | `claude-3-5-sonnet-20241022` | 200K | Claude 3.5 Sonnet，支持思考 |
/// | `gemini-2.0-flash` | 1M | Gemini 2.0 Flash，支持思考 |
/// | `deepseek-chat` | 64K | DeepSeek V3 |
/// | `deepseek-reasoner` | 64K | DeepSeek R1，支持思考 |
///
/// # 模型别名
///
/// | 别名 | 解析为 |
/// |------|--------|
/// | `packycode` / `4o-mini` | `gpt-4o-mini` |
/// | `4o` | `gpt-4o` |
/// | `4.1` | `gpt-4.1` |
/// | `sonnet` / `claude` | `claude-sonnet-4-5-20250929` |
/// | `gemini` | `gemini-2.0-flash` |
/// | `deepseek` | `deepseek-chat` |
/// | `r1` | `deepseek-reasoner` |
///
/// # 使用示例
///
/// ```rust
/// use nl_llm_v2::LlmClient;
///
/// let client = LlmClient::from_preset("packycode")
///     .expect("Preset should exist")
///     .with_api_key("sk-xxx")
///     .build();
/// ```
///
/// # 获取 API Key
///
/// 访问 PackyCode 官网获取 API Key
const PACKYCODE_BASE_URL: &str = "https://api.packycode.com/v1";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(PACKYCODE_BASE_URL))
        .protocol(OpenAiProtocol {})
        .model_resolver(PackyCodeModelResolver::new())
        .with_extension(Arc::new(
            PackyCodeExtension::new().with_base_url(PACKYCODE_BASE_URL),
        ))
        .default_model("gpt-4o-mini")
}
