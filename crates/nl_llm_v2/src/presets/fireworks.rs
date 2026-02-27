use crate::client::ClientBuilder;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::site::base::openai::OpenAiSite;

/// Fireworks API 预设
///
/// Fireworks AI 是一个高性能的开源模型推理平台。
/// 使用标准 OpenAI 兼容协议，支持 Bearer Token 认证。
///
/// # 平台特性
///
/// - **端点**: `https://api.fireworks.ai/inference/v1`
/// - **认证**: `Authorization: Bearer <FIREWORKS_API_KEY>`
/// - **协议**: OpenAI 兼容
/// - **特色**: 支持大量前沿开源大模型，低延迟高并发
///
/// # 基本用法
///
/// ```rust,no_run
/// use nl_llm_v2::{LlmClient, PrimitiveRequest};
///
/// let client = LlmClient::from_preset("fireworks")
///     .expect("Preset should exist")
///     .with_api_key("fw_xxx")
///     .build();
///
/// let req = PrimitiveRequest::single_user_message("Hello")
///     .with_model("accounts/fireworks/models/llama-v3p3-70b-instruct");
/// ```
const FIREWORKS_BASE_URL: &str = "https://api.fireworks.ai/inference/v1";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(FIREWORKS_BASE_URL))
        .protocol(OpenAiProtocol)
        .default_model("accounts/fireworks/models/llama-v3p3-70b-instruct")
}
