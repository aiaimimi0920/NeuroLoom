//! Gemini Provider 实现
//!
//! 支持 API Key 认证，统一处理官方端点和转发站
//!
//! URL 格式:
//! - 官方: `https://generativelanguage.googleapis.com/v1beta/models/{model}:{action}`
//! - 转发站: `{base_url}/v1beta/models/{model}:{action}` 或自定义路径
//!
//! 认证方式: `x-goog-api-key` header

use super::protocol::{compile_request, parse_response, parse_sse_stream};
use super::config::GeminiConfig;
use crate::auth::{Auth, ApiKeyConfig, ApiKeyProvider};
use crate::primitive::PrimitiveRequest;
use crate::provider::{BoxStream, LlmChunk, LlmResponse, GenericClient, Endpoint, Protocol};
use crate::generic_client;
use async_trait::async_trait;

// ── 常量 ────────────────────────────────────────────────────────────────────────
const GOOGLE_AI_STUDIO_API_VERSION: &str = "v1beta";

// ── Orthogonal Decomposition: Protocol & Endpoint ─────────────────────────────

pub struct GeminiProtocol;

impl Protocol for GeminiProtocol {
    fn compile(&self, primitive: &PrimitiveRequest) -> serde_json::Value {
        compile_request(primitive)
    }

    fn parse_response(&self, raw_text: &str) -> crate::Result<LlmResponse> {
        parse_response(raw_text)
    }

    fn parse_stream(&self, resp: reqwest::Response) -> crate::Result<BoxStream<'static, crate::Result<LlmChunk>>> {
        Ok(parse_sse_stream(resp))
    }
}

pub struct GeminiEndpoint {
    base_url: String,
    auth_key: String,
    extra_headers: std::collections::HashMap<String, String>,
}

#[async_trait]
impl Endpoint for GeminiEndpoint {
    async fn pre_flight(&self) -> crate::Result<()> {
        Ok(())
    }

    fn url(&self, model: &str, is_stream: bool) -> crate::Result<String> {
        let action = if is_stream { "streamGenerateContent?alt=sse" } else { "generateContent" };
        Ok(format!(
            "{}/{}/models/{}:{}",
            self.base_url.trim_end_matches('/'),
            GOOGLE_AI_STUDIO_API_VERSION,
            model,
            action
        ))
    }

    fn inject_auth(&self, mut req: reqwest::RequestBuilder) -> crate::Result<reqwest::RequestBuilder> {
        req = req.header("x-goog-api-key", &self.auth_key);
        for (k, v) in &self.extra_headers {
            req = req.header(k, v);
        }
        Ok(req)
    }
}

// ── GeminiProvider 本质是 GenericClient 装配好的别名 ──────────────────────
pub type GeminiProvider = GenericClient<GeminiEndpoint, GeminiProtocol>;

impl GeminiProvider {
    /// 使用外部指定的 HTTP Client 创建 Provider
    ///
    /// 注意：根据设计规范，HTTP Client 应由外部统一管理，
    /// 避免每个 Provider 重复创建连接池。
    pub fn new(config: GeminiConfig, http: reqwest::Client) -> Self {
        let auth_enum = Auth::ApiKey(ApiKeyConfig {
            key: config.auth.key.clone(),
            base_url: config.auth.base_url.clone(),
            provider: ApiKeyProvider::GeminiAIStudio,
        });

        let endpoint = GeminiEndpoint {
            base_url: config.base_url(),
            auth_key: config.auth.key.clone(),
            extra_headers: config.extra_headers.clone(),
        };

        generic_client! {
            id: "gemini".to_string(),
            endpoint: endpoint,
            protocol: GeminiProtocol,
            auth: auth_enum,
            supported_models: vec![
                "gemini-1.5-pro".to_string(),
                "gemini-1.5-flash".to_string(),
                "gemini-2.0-flash".to_string(),
                "gemini-2.5-flash".to_string(),
                "gemini-2.5-pro".to_string(),
            ],
            http: http
        }
    }

    /// 使用 API Key 创建 Provider（官方端点）
    ///
    /// 注意：此方法会创建新的 HTTP Client，推荐使用 `new()` 方法传入共享的 Client。
    pub fn from_api_key(api_key: impl Into<String>, model: impl Into<String>, http: reqwest::Client) -> Self {
        Self::new(GeminiConfig::with_api_key(api_key, model), http)
    }

    /// 使用 API Key 和自定义 Base URL 创建 Provider（转发站）
    pub fn from_api_key_with_base_url(
        api_key: impl Into<String>,
        base_url: impl Into<String>,
        model: impl Into<String>,
        http: reqwest::Client,
    ) -> Self {
        Self::new(GeminiConfig::with_api_key_and_base_url(api_key, base_url, model), http)
    }

    /// 获取当前配置的 Base URL
    pub fn base_url(&self) -> &str {
        &self.endpoint.base_url
    }

    /// 是否使用官方端点
    pub fn is_official(&self) -> bool {
        self.endpoint.base_url == "https://generativelanguage.googleapis.com"
    }
}

// ── GoogleAIStudioProvider 别名 ─────────────────────────────────────────────────
// 保持向后兼容

/// Google AI Studio Provider (GeminiProvider 别名)
///
/// 保留此类型以保持向后兼容
pub type GoogleAIStudioProvider = GeminiProvider;

// ── 测试 ─────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_client() -> reqwest::Client {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client")
    }

    #[test]
    fn test_from_api_key() {
        let http = create_test_client();
        let provider =
            GeminiProvider::from_api_key("AIzaSyTestKey".to_string(), "gemini-2.5-pro".to_string(), http);
        assert!(provider.is_official());
    }

    #[test]
    fn test_from_api_key_with_base_url() {
        let http = create_test_client();
        let provider = GeminiProvider::from_api_key_with_base_url(
            "test-key".to_string(),
            "https://zenmux.ai/api".to_string(),
            "gemini-2.5-flash".to_string(),
            http,
        );
        assert!(!provider.is_official());
    }

    #[test]
    fn test_build_url() {
        let http = create_test_client();
        let provider =
            GeminiProvider::from_api_key("test-key".to_string(), "gemini-2.5-flash".to_string(), http);
        assert_eq!(
            provider.endpoint.url("gemini-2.5-flash", false).unwrap(),
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent"
        );
    }

    #[test]
    fn test_build_url_custom_base() {
        let http = create_test_client();
        let provider = GeminiProvider::from_api_key_with_base_url(
            "test-key".to_string(),
            "https://zenmux.ai/api".to_string(),
            "gemini-2.5-flash".to_string(),
            http,
        );
        assert_eq!(
            provider.endpoint.url("gemini-2.5-flash", false).unwrap(),
            "https://zenmux.ai/api/v1beta/models/gemini-2.5-flash:generateContent"
        );
    }
}
