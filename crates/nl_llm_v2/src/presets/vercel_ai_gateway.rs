use crate::client::ClientBuilder;
use crate::model::OpenAiModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::site::base::openai::OpenAiSite;

/// Vercel AI Gateway API 预设
///
/// Vercel AI Gateway 是 Vercel 提供的统一 AI 接口网关，
/// 支持路由请求到多个模型提供商，并自带速率限制、缓存等功能。
///
/// # 平台特性
///
/// - **端点**: `https://ai-gateway.vercel.sh/v1`
/// - **认证**: `Authorization: Bearer <VERCEL_AI_KEY>`
/// - **协议**: 官方提供 OpenAI 兼容的统一网关协议端点
///
/// # 基本用法
///
/// ```rust,no_run
/// use nl_llm_v2::{LlmClient, PrimitiveRequest};
///
/// let client = LlmClient::from_preset("vercel_ai_gateway")
///     .expect("Preset should exist")
///     .with_api_key("vck_xxx")
///     .build();
///
/// let req = PrimitiveRequest::single_user_message("Hello")
///     .with_model("gpt-4o-mini"); // 取决于在 Vercel 后台配置的 Provider
/// ```
const VERCEL_AI_GATEWAY_BASE_URL: &str = "https://ai-gateway.vercel.sh/v1";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(VERCEL_AI_GATEWAY_BASE_URL))
        .protocol(OpenAiProtocol)
        .model_resolver(OpenAiModelResolver::new())
        .default_model("gpt-4o") // 后端通常会映射
}
