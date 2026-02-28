use std::sync::Arc;

use crate::auth::providers::ApiKeyAuth;
use crate::client::{ClientBuilder, LlmClient};
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::qiniu::{QiniuExtension, QiniuModelResolver};
use crate::site::base::openai::OpenAiSite;

const QINIU_BASE_URL: &str = "https://ai.qiniuapi.com/v1";

/// 七牛云 AI (Qiniu AI) 预设
///
/// 七牛云提供了兼容 OpenAI 格式的推理接口。
/// 默认接入点为 `https://ai.qiniuapi.com/v1`。
pub fn builder() -> ClientBuilder {
    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(QINIU_BASE_URL))
        .protocol(OpenAiProtocol {})
        .model_resolver(QiniuModelResolver::new())
        .with_extension(Arc::new(
            QiniuExtension::new().with_base_url(QINIU_BASE_URL),
        ))
        .default_model("qwen-plus")
}

impl LlmClient {
    /// 便捷构造七牛云 AI 客户端
    pub fn build_qiniu(api_key: impl Into<String>) -> Self {
        builder().auth(ApiKeyAuth::new(api_key)).build()
    }
}
