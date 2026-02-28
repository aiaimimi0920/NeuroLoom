use std::sync::Arc;

use crate::auth::providers::ApiKeyAuth;
use crate::client::{ClientBuilder, LlmClient};
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::burncloud::{BurnCloudExtension, BurnCloudModelResolver};
use crate::site::base::openai::OpenAiSite;

/// BurnCloud 预设
///
/// BurnCloud 作为 OpenAI API 协议层的一种代理聚合服务，
/// 连接机制非常类似于 OneAPI、NewAPI。
/// 此预设会尝试从 `BURNCLOUD_BASE_URL` 获取用户配置的热点端点；
/// 若未配置，默认回退至 `https://api.burn.hair/v1`。
pub fn builder() -> ClientBuilder {
    let base_url = read_burncloud_base_url();

    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(&base_url))
        .protocol(OpenAiProtocol {})
        .model_resolver(BurnCloudModelResolver::new())
        .with_extension(Arc::new(BurnCloudExtension::new(&base_url)))
}

fn read_burncloud_base_url() -> String {
    std::env::var("BURNCLOUD_BASE_URL")
        .ok()
        .map(|s| s.trim().trim_end_matches('/').to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "https://api.burn.hair/v1".to_string())
}

impl LlmClient {
    /// 便捷构造 BurnCloud 客户端
    pub fn build_burncloud(api_key: impl Into<String>) -> Self {
        builder().auth(ApiKeyAuth::new(api_key)).build()
    }

    /// 借助自定义端点便捷构造 BurnCloud 客户端
    pub fn build_burncloud_with_url(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        let base_url = base_url.into();
        builder()
            .site(OpenAiSite::new().with_base_url(&base_url))
            .auth(ApiKeyAuth::new(api_key))
            .with_extension(Arc::new(BurnCloudExtension::new(&base_url)))
            .build()
    }
}
