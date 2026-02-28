use crate::client::ClientBuilder;
use crate::model::doubao::DoubaoModelResolver;
use crate::provider::doubao::DoubaoExtension;
use crate::site::base::doubao::DoubaoSite;
use std::sync::Arc;

/// Doubao Video 预设
///
/// 豆包视频大模型（基于火山引擎），使用 Bearer Api Key 鉴权
pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(DoubaoSite::new())
        .model_resolver(DoubaoModelResolver::new())
        .with_extension(Arc::new(DoubaoExtension::new()))
        .protocol(crate::protocol::base::openai::OpenAiProtocol)
        .default_model("ep-20250226210255-h4249")
}
