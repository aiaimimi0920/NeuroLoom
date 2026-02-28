use std::sync::Arc;

use crate::auth::providers::ApiKeyAuth;
use crate::client::{ClientBuilder, LlmClient};
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::voyage::{VoyageExtension, VoyageModelResolver};
use crate::site::base::openai::OpenAiSite;

/// Voyage AI 预设
///
/// Voyage AI 专精于大模型的 Embedding 和 Reranking 能力分析，
/// 后端接口完全兼容 OpenAI 的 `/v1/embeddings` 请求格式。
pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url("https://api.voyageai.com/v1"))
        .protocol(OpenAiProtocol {})
        .model_resolver(VoyageModelResolver::new())
        .with_extension(Arc::new(VoyageExtension::new()))
}

impl LlmClient {
    /// 便捷构造 Voyage AI 客户端
    pub fn build_voyage(api_key: impl Into<String>) -> Self {
        builder().auth(ApiKeyAuth::new(api_key)).build()
    }
}
