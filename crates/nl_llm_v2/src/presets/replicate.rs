use std::sync::Arc;
use crate::client::ClientBuilder;
use crate::site::base::replicate::ReplicateSite;
use crate::model::replicate::ReplicateModelResolver;
use crate::provider::replicate::ReplicateExtension;

/// Replicate 预设
///
/// 取用 Replicate API, 需提供 Bearer Api Key 鉴权
pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(ReplicateSite::new())
        .model_resolver(ReplicateModelResolver::new())
        .with_extension(Arc::new(ReplicateExtension::new()))
        .default_model("minimax/video-01")
}
