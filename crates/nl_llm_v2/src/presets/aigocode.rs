use crate::client::ClientBuilder;
use crate::model::aigocode::AiGoCodeModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::aigocode::AiGoCodeExtension;
use crate::site::base::openai::OpenAiSite;
use std::sync::Arc;

/// AIGoCode AI 编程助手平台预设
///
/// AIGoCode 提供稳定高效的 AI 编程服务，聚合多家主流模型提供商。
///
/// # 平台特性
///
/// - **网关**: `https://api.aigocode.com/v1`
/// - **认证**: `Authorization: Bearer <API_KEY>`，密钥 `sk-` 前缀
/// - **协议**: OpenAI 兼容
/// - **并发**: 官方上限 5，初始并发 3
///
/// # 支持的模型
///
/// | 模型 ID | 说明 | 上下文长度 |
/// |---------|------|-----------|
/// | `claude-sonnet-4-5-20250929` | Claude Sonnet 4.5 | 200K |
/// | `claude-3-5-sonnet-20241022` | Claude 3.5 Sonnet | 200K |
/// | `gpt-4o` | GPT-4o | 128K |
/// | `gpt-4o-mini` | GPT-4o Mini | 128K |
/// | `gemini-2.0-flash` | Gemini 2.0 Flash | 1M |
/// | `deepseek-chat` | DeepSeek V3 | 64K |
/// | `deepseek-reasoner` | DeepSeek R1 推理模型 | 64K |
///
/// # 可用别名
///
/// | 别名 | 解析为 | 说明 |
/// |------|--------|------|
/// | `aigocode` / `sonnet` / `claude` | `claude-sonnet-4-5-20250929` | 默认模型 |
/// | `4o` | `gpt-4o` | GPT-4o |
/// | `4o-mini` | `gpt-4o-mini` | GPT-4o Mini |
/// | `gemini` | `gemini-2.0-flash` | Gemini 2.0 Flash |
/// | `deepseek` | `deepseek-chat` | DeepSeek V3 |
/// | `r1` | `deepseek-reasoner` | DeepSeek R1 推理模型 |
///
/// # 基本用法
///
/// ```rust,no_run
/// use nl_llm_v2::{LlmClient, PrimitiveRequest};
///
/// let client = LlmClient::from_preset("aigocode")
///     .expect("Preset should exist")
///     .with_api_key("sk-xxx")
///     .build();
///
/// // 使用默认模型
/// let req = PrimitiveRequest::single_user_message("你好!");
///
/// // 使用模型别名
/// let req = PrimitiveRequest::single_user_message("Write a hello world")
///     .with_model("4o");  // 自动解析为 gpt-4o
///
/// // 使用 DeepSeek R1 推理模型
/// let req = PrimitiveRequest::single_user_message("Solve this problem")
///     .with_model("r1");  // 自动解析为 deepseek-reasoner
/// ```
const AIGOCODE_BASE_URL: &str = "https://api.aigocode.com/v1";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(AIGOCODE_BASE_URL))
        .protocol(OpenAiProtocol {})
        .model_resolver(AiGoCodeModelResolver::new())
        .with_extension(Arc::new(
            AiGoCodeExtension::new().with_base_url(AIGOCODE_BASE_URL),
        ))
        .default_model("claude-sonnet-4-5-20250929")
}
