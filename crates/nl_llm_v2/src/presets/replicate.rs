use crate::client::ClientBuilder;
use crate::model::replicate::ReplicateModelResolver;
use crate::provider::replicate::ReplicateExtension;
use crate::site::base::replicate::ReplicateSite;
use std::sync::Arc;

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
