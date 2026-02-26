use crate::client::ClientBuilder;
use crate::site::base::openai::OpenAiSite;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::model::submodel::SubModelModelResolver;

/// SubModel API 预设
///
/// 完全兼容 OpenAI API，使用 Bearer Token 认证。
/// 端点: https://llm.submodel.ai/v1
///
const SUBMODEL_BASE_URL: &str = "https://llm.submodel.ai/v1";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(SUBMODEL_BASE_URL))
        .protocol(OpenAiProtocol {})
        .model_resolver(SubModelModelResolver::new())
        .default_model("submodel")  // 占位默认名称
}
