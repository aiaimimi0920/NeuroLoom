use crate::client::ClientBuilder;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::custom::{CustomExtension, CustomModelResolver};
use crate::site::base::openai::OpenAiSite;
use std::sync::Arc;

/// Hyperbolic API 预设
///
/// Hyperbolic 提供 OpenAI 兼容的大模型推理服务。
/// 使用标准 Bearer Token 认证，可通过 `/models` 动态获取可用模型。
///
/// # 平台特性
///
/// - **端点**: `https://api.hyperbolic.xyz/v1`
/// - **认证**: `Authorization: Bearer <HYPERBOLIC_API_KEY>`
/// - **协议**: OpenAI 兼容
/// - **特色**: 提供开源模型推理与统一 OpenAI 接口
///
/// # 基本用法
///
/// ```rust,no_run
/// use nl_llm::{LlmClient, PrimitiveRequest};
///
/// let client = LlmClient::from_preset("hyperbolic")
///     .expect("Preset should exist")
///     .with_api_key("your_key")
///     .build();
///
/// let req = PrimitiveRequest::single_user_message("Hello")
///     .with_model("meta-llama/Meta-Llama-3.1-70B-Instruct");
/// ```
const HYPERBOLIC_BASE_URL: &str = "https://api.hyperbolic.xyz/v1";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(HYPERBOLIC_BASE_URL))
        .protocol(OpenAiProtocol)
        .model_resolver(CustomModelResolver::new())
        .with_extension(Arc::new(CustomExtension::new(HYPERBOLIC_BASE_URL)))
        .default_model("meta-llama/Meta-Llama-3.1-70B-Instruct")
}
