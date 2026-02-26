use std::sync::Arc;

use crate::client::ClientBuilder;
use crate::model::vidu::ViduModelResolver;
use crate::provider::vidu::ViduExtension;
use crate::site::base::vidu::ViduSite;

/// Vidu 预设（v0）
///
/// - Base URL: https://api.vidu.cn
/// - Auth: Authorization: Token <VIDU_API_KEY>
/// - Video API: /ent/v2/img2video + /ent/v2/tasks/{id}/creations
///
/// 注意：鉴权需要使用 `ViduApiKeyAuth`（不是默认 Bearer ApiKeyAuth）。
pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(ViduSite::new())
        .model_resolver(ViduModelResolver::new())
        .with_extension(Arc::new(ViduExtension::new()))
        .protocol(crate::protocol::base::openai::OpenAiProtocol)
        .default_model("viduq1")
}
