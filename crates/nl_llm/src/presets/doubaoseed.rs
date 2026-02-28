use crate::client::ClientBuilder;
use crate::model::doubaoseed::DouBaoSeedModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::doubaoseed::DouBaoSeedExtension;
use crate::site::base::openai::OpenAiSite;
use std::sync::Arc;

/// DouBaoSeed (字节跳动 · 豆包) 预设
///
/// - **网关**: `https://ark.cn-beijing.volces.com/api/v3`
/// - **协议**: OpenAI 兼容
const DOUBAOSEED_BASE_URL: &str = "https://ark.cn-beijing.volces.com/api/v3";

pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(DOUBAOSEED_BASE_URL))
        .protocol(OpenAiProtocol {})
        .model_resolver(DouBaoSeedModelResolver::new())
        .with_extension(Arc::new(DouBaoSeedExtension::new()))
        .default_model("doubao-seed-2-0-pro-260215")
}
