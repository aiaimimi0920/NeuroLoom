use crate::client::ClientBuilder;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::site::base::openai::OpenAiSite;

/// Together API 预设
///
/// Together 是一个极速的开源 AI 模型推理平台。
/// 使用标准 OpenAI 兼容协议，支持 Bearer Token 认证。
///
/// # 平台特性
///
/// - **端点**: `https://api.together.xyz/v1`
/// - **认证**: `Authorization: Bearer <TOGETHER_API_KEY>`
/// - **协议**: OpenAI 兼容
/// - **特色**: 支持大量主流开源大模型，推理速度极快
///
/// # 基本用法
///
/// ```rust,no_run
/// use nl_llm_v2::{LlmClient, PrimitiveRequest};
///
/// let client = LlmClient::from_preset("together")
///     .expect("Preset should exist")
///     .with_api_key("key_xxx")
///     .build();
///
/// let req = PrimitiveRequest::single_user_message("Hello")
///     .with_model("meta-llama/Llama-3.3-70B-Instruct-Turbo");
/// ```
const TOGETHER_BASE_URL: &str = "https://api.together.xyz/v1";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(TOGETHER_BASE_URL))
        .protocol(OpenAiProtocol)
        .default_model("meta-llama/Llama-3.3-70B-Instruct-Turbo")
}
