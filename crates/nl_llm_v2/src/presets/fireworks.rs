use crate::client::ClientBuilder;
use crate::model::OpenAiModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::openai::OpenAiExtension;
use crate::site::base::openai::OpenAiSite;
use std::sync::Arc;

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
        // Fireworks 完整兼容 OpenAI Chat/Tool schema，直接复用 OpenAI 解析能力。
        .model_resolver(OpenAiModelResolver::new())
        // 复用 OpenAI 通用扩展接口（如 list_models），保持预设能力一致性。
        .with_extension(Arc::new(OpenAiExtension::new()))
        .default_model("accounts/fireworks/models/llama-v3p3-70b-instruct")
}
