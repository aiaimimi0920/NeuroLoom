use crate::auth::providers::ApiKeyAuth;
use crate::client::{ClientBuilder, LlmClient};
use crate::protocol::base::openai::OpenAiProtocol;
use crate::provider::gpustack::{GpuStackExtension, GpuStackModelResolver};
use crate::site::base::openai::OpenAiSite;
use std::sync::Arc;

fn normalize_base_url(base_url: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');
    if trimmed.ends_with("/v1") {
        trimmed.to_string()
    } else {
        format!("{}/v1", trimmed)
    }
}

/// GPUStack 预设
///
/// GPUStack 是一个用于部署和管理本地开源大模型（如 LLaMA, Qwen）的开源引擎框架，
/// 提供与 OpenAI 完全兼容的 REST API。
///
/// 环境变量 `GPUSTACK_BASE_URL` 可用于指定集群的地址，默认为 `"http://127.0.0.1:8080/v1"`
pub fn builder() -> ClientBuilder {
    let raw_base_url = std::env::var("GPUSTACK_BASE_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8080/v1".to_string());
    let base_url = normalize_base_url(&raw_base_url);

    ClientBuilder::new()
        .site(OpenAiSite::new().with_base_url(&base_url))
        .protocol(OpenAiProtocol {})
        .model_resolver(GpuStackModelResolver::new())
        .with_extension(Arc::new(GpuStackExtension::new(base_url.clone())))
}

/// 便捷构建器
///
/// 支持指定自定义局域网 Base URL 以及对应的 API Key 快速启动请求客户端。
impl LlmClient {
    pub fn build_gpustack(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        let base_url = normalize_base_url(&base_url.into());
        builder()
            .site(OpenAiSite::new().with_base_url(&base_url))
            .auth(ApiKeyAuth::new(api_key))
            .with_extension(Arc::new(GpuStackExtension::new(&base_url)))
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_base_url;

    #[test]
    fn normalize_keeps_v1_url() {
        assert_eq!(
            normalize_base_url("http://127.0.0.1:8080/v1"),
            "http://127.0.0.1:8080/v1"
        );
    }

    #[test]
    fn normalize_appends_v1_when_missing() {
        assert_eq!(
            normalize_base_url("http://127.0.0.1:8080"),
            "http://127.0.0.1:8080/v1"
        );
    }
}
