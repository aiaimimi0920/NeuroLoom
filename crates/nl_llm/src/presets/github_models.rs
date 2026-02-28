use crate::client::ClientBuilder;
use crate::model::github_models::GitHubModelsModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::site::base::openai::OpenAiSite;

/// GitHub Models API 预设
///
/// GitHub Models 是 GitHub 提供的 AI 模型推理服务，
/// 允许开发者在 GitHub 生态中直接使用各类前沿大模型。
///
/// # 平台特性
///
/// - **端点**: `https://models.inference.ai.azure.com`
/// - **认证**: `Authorization: Bearer <GITHUB_TOKEN>`
/// - **协议**: OpenAI 兼容
/// - **模型 ID**: 推荐 `provider/model` 形式（如 `openai/gpt-4o-mini`）
///
/// # 基本用法
///
/// ```rust,no_run
/// use nl_llm::{LlmClient, PrimitiveRequest};
///
/// let client = LlmClient::from_preset("github_models")
///     .expect("Preset should exist")
///     .with_api_key("github_pat_xxx")
///     .build();
///
/// let req = PrimitiveRequest::single_user_message("Hello")
///     .with_model("openai/gpt-4o-mini");
/// ```
const GITHUB_MODELS_BASE_URL: &str = "https://models.inference.ai.azure.com";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(GITHUB_MODELS_BASE_URL))
        .protocol(OpenAiProtocol)
        .model_resolver(GitHubModelsModelResolver::new())
        .default_model("openai/gpt-4o-mini")
}
