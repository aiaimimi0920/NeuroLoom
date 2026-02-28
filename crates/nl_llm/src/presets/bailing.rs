use crate::client::ClientBuilder;
use crate::model::bailing::BaiLingModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::bailing::BaiLingExtension;
use crate::site::base::openai::OpenAiSite;
use std::sync::Arc;

/// BaiLing (百灵) 预设
///
/// - **网关**: `https://api.tbox.cn/api/llm/v1`
/// - **协议**: OpenAI 兼容
/// - **默认模型**: `Ling-1T`
const BAILING_BASE_URL: &str = "https://api.tbox.cn/api/llm/v1";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(BAILING_BASE_URL))
        .protocol(OpenAiProtocol {})
        .model_resolver(BaiLingModelResolver::new())
        .with_extension(Arc::new(BaiLingExtension::new()))
        .default_model("Ling-1T")
}
