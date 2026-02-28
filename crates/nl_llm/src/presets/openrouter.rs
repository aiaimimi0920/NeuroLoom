use crate::client::ClientBuilder;
use crate::model::openrouter::OpenRouterModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::openrouter;
use crate::site::base::openai::OpenAiSite;

/// OpenRouter 聚合平台预设
///
/// # 平台特性
///
/// - **网关**: `https://openrouter.ai/api/v1`
/// - **认证**: 标准 Bearer 格式
/// - **协议**: OpenAI 兼容
/// - **类型**: API 聚合平台，支持多种模型
///
/// # 模型命名规则
///
/// OpenRouter 模型 ID 格式: `provider/model-name`
///
/// ## 模型变体
///
/// | 后缀 | 说明 |
/// |------|------|
/// | `:free` | 免费模型 |
/// | `:extended` | 扩展上下文窗口 |
/// | `:thinking` | 扩展推理能力 |
/// | `:online` | 实时网络搜索 |
/// | `:nitro` | 高速推理 |
///
/// # 支持的模型（部分）
///
/// | 模型 ID | 上下文 | 说明 |
/// |---------|--------|------|
/// | `anthropic/claude-3.5-sonnet` | 200K | Claude 3.5 Sonnet |
/// | `anthropic/claude-3.5-haiku` | 200K | Claude 3.5 Haiku |
/// | `openai/gpt-4o` | 128K | GPT-4o |
/// | `google/gemini-2.5-pro` | 1M | Gemini 2.5 Pro |
/// | `deepseek/deepseek-chat` | 64K | DeepSeek Chat |
///
/// # 使用示例
///
/// ```rust
/// use nl_llm::LlmClient;
///
/// let client = LlmClient::from_preset("openrouter")
///     .expect("Preset should exist")
///     .with_api_key("your-api-key")
///     .build();
/// ```
const OPENROUTER_BASE_URL: &str = "https://openrouter.ai/api/v1";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(OPENROUTER_BASE_URL))
        .protocol(OpenAiProtocol {})
        .model_resolver(OpenRouterModelResolver::new())
        .with_extension(openrouter::extension())
        .default_model("google/gemini-2.5-flash")
}
