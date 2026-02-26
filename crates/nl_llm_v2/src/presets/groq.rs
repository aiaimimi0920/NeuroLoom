use crate::client::ClientBuilder;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::site::base::openai::OpenAiSite;

/// Groq API 预设
///
/// Groq 是全球最快的 LLM 推理平台，使用自研 LPU（Language Processing Unit）芯片。
/// 使用标准 OpenAI 兼容协议，支持 Bearer Token 认证。
///
/// # 平台特性
///
/// - **端点**: `https://api.groq.com/openai/v1`
/// - **认证**: `Authorization: Bearer <GROQ_API_KEY>`
/// - **协议**: OpenAI 兼容
/// - **特色**: 超低延迟推理，支持 Llama、Mixtral、Gemma 等开源模型
///
/// # 基本用法
///
/// ```rust,no_run
/// use nl_llm_v2::{LlmClient, PrimitiveRequest};
///
/// let client = LlmClient::from_preset("groq")
///     .expect("Preset should exist")
///     .with_api_key("gsk_xxx")
///     .build();
///
/// let req = PrimitiveRequest::single_user_message("Hello")
///     .with_model("llama-3.3-70b-versatile");
/// ```
const GROQ_BASE_URL: &str = "https://api.groq.com/openai/v1";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(GROQ_BASE_URL))
        .protocol(OpenAiProtocol)
        .default_model("llama-3.3-70b-versatile")
}
