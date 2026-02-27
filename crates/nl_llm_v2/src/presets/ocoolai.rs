use crate::client::ClientBuilder;
use crate::model::ocoolai::OcoolAiModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::ocoolai::OcoolAiExtension;
use crate::site::base::openai::OpenAiSite;
use std::sync::Arc;

/// ocoolAI 聚合平台预设
///
/// ocoolAI 是一个 AI API 聚合平台，提供 OpenAI 兼容接口，
/// 支持 200+ 大语言模型，包括 GPT、Claude、Gemini、DeepSeek 等。
///
/// # 平台特性
///
/// - **网关**: `https://api.ocoolai.com/v1`
/// - **认证**: 标准 Bearer 格式，API Key 以 `sk-` 为前缀
/// - **协议**: OpenAI 兼容
/// - **类型**: AI 模型 API 中转服务平台
///
/// # 支持的模型（热门）
///
/// | 模型 ID | 上下文 | 说明 |
/// |---------|--------|------|
/// | `gpt-4o` | 128K | GPT-4o，支持视觉 |
/// | `gpt-4o-mini` | 128K | GPT-4o Mini |
/// | `gpt-4-turbo` | 128K | GPT-4 Turbo |
/// | `gpt-3.5-turbo` | 16K | GPT-3.5 Turbo |
/// | `claude-3-5-sonnet-20241022` | 200K | Claude 3.5 Sonnet |
/// | `claude-3-opus-20240229` | 200K | Claude 3 Opus |
/// | `gemini-1.5-pro` | 1M | Gemini 1.5 Pro |
/// | `gemini-1.5-flash` | 1M | Gemini 1.5 Flash |
/// | `deepseek-chat` | 64K | DeepSeek V3 |
/// | `deepseek-reasoner` | 64K | DeepSeek R1，支持思考 |
///
/// # 可用别名
///
/// | 别名 | 解析为 |
/// |------|--------|
/// | `ocool` / `4o-mini` | `gpt-4o-mini` |
/// | `4o` | `gpt-4o` |
/// | `claude` / `sonnet` | `claude-3-5-sonnet-20241022` |
/// | `gemini` | `gemini-1.5-flash` |
/// | `deepseek` / `ds` | `deepseek-chat` |
/// | `r1` / `think` | `deepseek-reasoner` |
///
/// # 使用示例
///
/// ```rust,no_run
/// use nl_llm_v2::{LlmClient, PrimitiveRequest};
///
/// let client = LlmClient::from_preset("ocoolai")
///     .expect("Preset should exist")
///     .with_api_key("sk-xxx")
///     .build();
///
/// // 使用别名
/// let req = PrimitiveRequest::single_user_message("Hello!")
///     .with_model("4o");  // 解析为 gpt-4o
/// ```
///
/// # 获取 API Key
///
/// 访问 https://api.ocoolai.com 注册获取 API Key
const OCOOLAI_BASE_URL: &str = "https://api.ocoolai.com/v1";

pub fn builder() -> ClientBuilder {
    let base_url = std::env::var("OCOOLAI_BASE_URL")
        .unwrap_or_else(|_| OCOOLAI_BASE_URL.to_string());

    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(&base_url))
        .protocol(OpenAiProtocol {})
        .model_resolver(OcoolAiModelResolver::new())
        .with_extension(Arc::new(
            OcoolAiExtension::new().with_base_url(&base_url),
        ))
        .default_model("gpt-4o-mini")
}
