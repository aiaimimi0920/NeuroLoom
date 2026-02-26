use std::sync::Arc;

use crate::client::ClientBuilder;
use crate::model::ViduModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::vidu::ViduExtension;
use crate::site::base::vidu::ViduSite;

/// Vidu 官方 API 预设
///
/// - Base URL: `https://api.vidu.cn`
/// - Auth: `Authorization: Token <VIDU_API_KEY>`（见 [`crate::auth::providers::vidu::ViduAuth`](crate::auth::providers::vidu::ViduAuth)）
/// - 核心能力：视频异步任务（submit/fetch）
pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(ViduSite::new())
        // 为了兼容框架流水线的 Pack/Unpack 结构，这里仍注入 OpenAI 协议占位。
        // Vidu 的视频任务接口不使用该协议。
        .protocol(OpenAiProtocol {})
        .model_resolver(ViduModelResolver::new())
        .with_extension(Arc::new(ViduExtension::new()))
        .default_model("viduq1")
}
