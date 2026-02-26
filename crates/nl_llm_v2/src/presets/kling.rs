use crate::client::ClientBuilder;
use crate::site::base::kling::KlingSite;
use std::sync::Arc;
// 虽然 Kling 是视频 API，但出于兼容要求，我们还是注入一段可用的 openai 协议防止底层 Pack/Unpack 问题
use crate::model::KlingModelResolver;
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::kling::KlingExtension;

/// 可灵 AI (Kling) 预设
///
/// 官方 Base URL: `https://api.klingai.com`
/// 认证方式: JWT (基于 AccessKey 和 SecretKey 签名的流转令牌)
pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(KlingSite::new())
        .protocol(OpenAiProtocol {})
        .model_resolver(KlingModelResolver::new())
        .with_extension(Arc::new(KlingExtension::new()))
        .default_model("kling-v1")
}
