use crate::client::ClientBuilder;
use crate::model::CerebrasModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::custom::CustomExtension;
use crate::site::base::openai::OpenAiSite;
use std::sync::Arc;

/// CereBras AI 预设
///
/// CereBras 提供超快推理速度的 LLM API 服务，使用专有的 Wafer-Scale Engine (WSE) 芯片。
/// 使用 OpenAI 兼容协议，支持 Bearer Token 认证。
///
/// # 平台特性
///
/// - **端点**: `https://api.cerebras.ai/v1`
/// - **认证**: `Authorization: Bearer <CEREBRAS_API_KEY>`
/// - **协议**: OpenAI 兼容
///
/// # 模型说明
///
/// | 模型 | 说明 |
/// |------|------|
/// | `llama3.1-8b` | Llama 3.1 8B 推理模型 |
/// | `llama-3.3-70b` | Llama 3.3 70B 推理模型 |
///
/// # 基本用法
///
/// ```rust,no_run
/// use nl_llm::{LlmClient, PrimitiveRequest};
///
/// let client = LlmClient::from_preset("cerebras")
///     .expect("Preset should exist")
///     .with_api_key("csk-xxx")
///     .build();
///
/// let req = PrimitiveRequest::single_user_message("Hello")
///     .with_model("llama3.1-8b");
/// ```
const CEREBRAS_BASE_URL: &str = "https://api.cerebras.ai/v1";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(CEREBRAS_BASE_URL))
        .protocol(OpenAiProtocol {})
        .model_resolver(CerebrasModelResolver::new())
        .with_extension(Arc::new(CustomExtension::new(CEREBRAS_BASE_URL)))
        .default_model("llama3.1-8b")
}
