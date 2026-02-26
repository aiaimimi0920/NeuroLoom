use std::sync::Arc;
use crate::client::ClientBuilder;
use crate::site::base::sora::SoraSite;
use crate::model::sora::SoraModelResolver;
use crate::provider::sora::SoraExtension;

/// Sora Video 预设
///
/// 取用 OpenAI /v1/videos 标准协议，使用 Bearer Api Key 鉴权
pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(SoraSite::new())
        .model_resolver(SoraModelResolver::new())
        .with_extension(Arc::new(SoraExtension::new()))
        .default_model("sora")
}
