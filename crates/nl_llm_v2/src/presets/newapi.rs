use crate::auth::providers::ApiKeyAuth;
use crate::client::{ClientBuilder, LlmClient};
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::newapi::{NewApiExtension, NewApiModelResolver};
use crate::site::base::openai::OpenAiSite;
use std::sync::Arc;

/// NewAPI 预设
///
/// NewAPI 是一种兼容 OpenAI 格式的中转代理服务，通常运行在用户自定义地址。
/// 本预设可以连接到任何 NewAPI 兼容端点（如 Cherry Studio）。
///
/// 使用前请确保：
/// 1. 提供了自定义的 `base_url`（如 `"http://127.0.0.1:3000/v1"`）。
///    如果不通过 Builder 手动覆盖，默认尝试读取 `NEWAPI_BASE_URL` 环境变量。
/// 2. 提供了对应的 Token 进行认证 `auth(Auth::api_key("sk-xxx"))`。
pub fn builder() -> ClientBuilder {
    let base_url = std::env::var("NEWAPI_BASE_URL").unwrap_or_else(|_| "".to_string());

    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(&base_url))
        .protocol(OpenAiProtocol {})
        .model_resolver(NewApiModelResolver::new())
        .with_extension(Arc::new(NewApiExtension::new(base_url)))
}

/// 便捷构建器
/// 
/// 考虑到 NewAPI 作为自定义转发，经常需要显式注入 Base URL 与 API Key，提供该扩展实现。
impl LlmClient {
    pub fn build_newapi(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        let base_url = base_url.into();
        builder()
            .site(OpenAiSite::new().with_base_url(&base_url))
            .auth(ApiKeyAuth::new(api_key))
            .with_extension(Arc::new(NewApiExtension::new(&base_url)))
            .build()
    }
}
