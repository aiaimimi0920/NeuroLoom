use crate::client::ClientBuilder;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::site::base::openai::OpenAiSite;

/// PPIO API 预设
///
/// PPIO 是一个分布式算力平台，提供高性价比的 AI 模型推理服务。
/// 使用标准 OpenAI 兼容协议，支持 Bearer Token 认证。
///
/// # 平台特性
///
/// - **端点**: `https://api.ppio.com/openai`
/// - **认证**: `Authorization: Bearer <PPIO_API_KEY>`
/// - **协议**: OpenAI 兼容
/// - **特色**: 分布式算力，高性价比推理，支持主流开源模型
///
/// # 基本用法
///
/// ```rust,no_run
/// use nl_llm_v2::{LlmClient, PrimitiveRequest};
///
/// let client = LlmClient::from_preset("ppio")
///     .expect("Preset should exist")
///     .with_api_key("sk_xxx")
///     .build();
///
/// let req = PrimitiveRequest::single_user_message("Hello")
///     .with_model("deepseek/deepseek-v3/community");
/// ```
const PPIO_BASE_URL: &str = "https://api.ppio.com/openai";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(PPIO_BASE_URL))
        .protocol(OpenAiProtocol)
        .default_model("deepseek/deepseek-v3/community")
}
