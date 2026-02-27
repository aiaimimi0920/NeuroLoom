use crate::client::ClientBuilder;
use crate::model::InfiniModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::site::base::openai::OpenAiSite;

/// 无问芯穹 (Infinigence AI / GenStudio) 预设
///
/// 无问芯穹是一个国产 AI 推理云平台，提供多种大模型 API 服务。
/// 使用 OpenAI 兼容协议，支持 Bearer Token 认证。
///
/// # 平台特性
///
/// - **端点**: `https://cloud.infini-ai.com/maas/v1`
/// - **认证**: `Authorization: Bearer <API_KEY>`
/// - **协议**: OpenAI 兼容
///
/// # 基本用法
///
/// ```rust,no_run
/// use nl_llm_v2::{LlmClient, PrimitiveRequest};
///
/// let client = LlmClient::from_preset("infini")
///     .expect("Preset should exist")
///     .with_api_key("sk-xxx")
///     .build();
///
/// let req = PrimitiveRequest::single_user_message("Hello")
///     .with_model("your-model");
/// ```
const INFINI_BASE_URL: &str = "https://cloud.infini-ai.com/maas/v1";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(INFINI_BASE_URL))
        .protocol(OpenAiProtocol {})
        .model_resolver(InfiniModelResolver::new())
        .default_model("deepseek-v3")
}
