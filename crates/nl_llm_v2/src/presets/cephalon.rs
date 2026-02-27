use crate::client::ClientBuilder;
use crate::model::CephalonModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::cephalon::CephalonExtension;
use crate::site::base::openai::OpenAiSite;
use std::sync::Arc;

/// Cephalon API 预设
///
/// Cephalon 是一个 AI 模型聚合平台，提供多种 LLM 模型服务。
/// 使用 OpenAI 兼容协议，支持 Bearer Token 认证。
///
/// # 平台特性
///
/// - **端点**: `https://cephalon.cloud/user-center/v1/model`
/// - **认证**: `Authorization: Bearer <CEPHALON_API_KEY>`
/// - **协议**: OpenAI 兼容
///
/// # 基本用法
///
/// ```rust,no_run
/// use nl_llm_v2::{LlmClient, PrimitiveRequest};
///
/// let client = LlmClient::from_preset("cephalon")
///     .expect("Preset should exist")
///     .with_api_key("sk-xxx")
///     .build();
///
/// let req = PrimitiveRequest::single_user_message("Hello")
///     .with_model("gpt-4o");
///
/// let resp = client.complete(&req).await?;
/// ```
const CEPHALON_BASE_URL: &str = "https://cephalon.cloud/user-center/v1/model";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(CEPHALON_BASE_URL))
        .protocol(OpenAiProtocol {})
        .model_resolver(CephalonModelResolver::new())
        .with_extension(Arc::new(CephalonExtension::new()))
        .default_model("gpt-4o")
}
