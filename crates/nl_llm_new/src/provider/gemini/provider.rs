//! Gemini Provider 实现
//!
//! 支持 API Key 认证，统一处理官方端点和转发站
//!
//! URL 格式:
//! - 官方: `https://generativelanguage.googleapis.com/v1beta/models/{model}:{action}`
//! - 转发站: `{base_url}/v1beta/models/{model}:{action}` 或自定义路径
//!
//! 认证方式: `x-goog-api-key` header

use super::common::{compile_gemini_request, parse_gemini_response, parse_gemini_sse_stream};
use super::config::GeminiConfig;
use crate::auth::{Auth, ApiKeyConfig, ApiKeyProvider};
use crate::primitive::PrimitiveRequest;
use crate::provider::{BoxStream, LlmChunk, LlmProvider, LlmResponse};
use async_trait::async_trait;
use std::time::Duration;

// ── 常量 ────────────────────────────────────────────────────────────────────────
const GOOGLE_AI_STUDIO_API_VERSION: &str = "v1beta";

// ── GeminiProvider ───────────────────────────────────────────────────────────────

/// Gemini Provider
///
/// 通过 API Key 认证，调用 Google AI Studio 或第三方转发站
/// - 官方端点: 不设置 base_url
/// - 转发站: 设置自定义 base_url
pub struct GeminiProvider {
    config: GeminiConfig,
    http: reqwest::Client,
    auth_enum: Auth,
}

impl GeminiProvider {
    /// 创建新的 Gemini Provider
    pub fn new(config: GeminiConfig) -> Self {
        Self::with_client(
            config,
            reqwest::Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .expect("Failed to create HTTP client"),
        )
    }

    /// 使用外部指定的 HTTP Client 创建 Provider
    pub fn with_client(config: GeminiConfig, http: reqwest::Client) -> Self {
        let auth_enum = Auth::ApiKey(ApiKeyConfig {
            key: config.auth.key.clone(),
            base_url: config.auth.base_url.clone(),
            provider: ApiKeyProvider::GeminiAIStudio,
        });

        Self {
            config,
            http,
            auth_enum,
        }
    }

    /// 使用 API Key 创建 Provider（官方端点）
    pub fn from_api_key(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self::new(GeminiConfig::with_api_key(api_key, model))
    }

    /// 使用 API Key 和自定义 Base URL 创建 Provider（转发站）
    pub fn from_api_key_with_base_url(
        api_key: impl Into<String>,
        base_url: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self::new(GeminiConfig::with_api_key_and_base_url(api_key, base_url, model))
    }

    /// 是否为官方端点
    pub fn is_official(&self) -> bool {
        self.config.is_official()
    }

    /// 构造 API URL
    fn build_url(&self, model: &str, action: &str) -> String {
        format!(
            "{}/{}/models/{}:{}",
            self.config.base_url().trim_end_matches('/'),
            GOOGLE_AI_STUDIO_API_VERSION,
            model,
            action
        )
    }
}

#[async_trait]
impl LlmProvider for GeminiProvider {
    fn id(&self) -> &str {
        "gemini"
    }

    fn auth(&self) -> &Auth {
        &self.auth_enum
    }

    fn supported_models(&self) -> &[&str] {
        &[
            "gemini-1.5-pro",
            "gemini-1.5-flash",
            "gemini-2.0-flash",
            "gemini-2.5-flash",
            "gemini-2.5-pro",
        ]
    }

    fn compile(&self, primitive: &PrimitiveRequest) -> serde_json::Value {
        compile_gemini_request(primitive)
    }

    async fn complete(&self, mut body: serde_json::Value) -> crate::Result<LlmResponse> {
        let model = body.as_object_mut()
            .and_then(|obj| obj.remove("_model"))
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| self.config.model.clone());

        let url = self.build_url(&model, "generateContent");

        let mut req = self
            .http
            .post(&url)
            .header("Content-Type", "application/json")
            .header("x-goog-api-key", &self.config.auth.key);

        for (k, v) in &self.config.extra_headers {
            req = req.header(k, v);
        }

        let resp = req
            .json(&body)
            .send()
            .await
            .map_err(|e| crate::Error::Http(e.to_string()))?;

        let status = resp.status();
        let raw_text = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(crate::Error::Provider(
                crate::provider::ProviderError::from_http_status(
                    status.as_u16(),
                    format!(
                        "gemini: generateContent failed ({}): {}",
                        status,
                        raw_text.trim()
                    ),
                ),
            ));
        }

        parse_gemini_response(&raw_text)
    }

    async fn stream(
        &self,
        mut body: serde_json::Value,
    ) -> crate::Result<BoxStream<'_, crate::Result<LlmChunk>>> {
        let model = body.as_object_mut()
            .and_then(|obj| obj.remove("_model"))
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| self.config.model.clone());

        let url = format!("{}?alt=sse", self.build_url(&model, "streamGenerateContent"));

        let mut req = self
            .http
            .post(&url)
            .header("Content-Type", "application/json")
            .header("x-goog-api-key", &self.config.auth.key)
            .header("Accept", "text/event-stream");

        for (k, v) in &self.config.extra_headers {
            req = req.header(k, v);
        }

        let resp = req
            .json(&body)
            .send()
            .await
            .map_err(|e| crate::Error::Http(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(crate::Error::Provider(
                crate::provider::ProviderError::from_http_status(
                    status.as_u16(),
                    format!(
                        "gemini: streamGenerateContent failed ({}): {}",
                        status,
                        text.trim()
                    ),
                ),
            ));
        }

        Ok(parse_gemini_sse_stream(resp))
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

    #[test]
    fn test_from_api_key() {
        let provider =
            GeminiProvider::from_api_key("AIzaSyTestKey".to_string(), "gemini-2.5-pro".to_string());
        assert_eq!(provider.config.auth.key, "AIzaSyTestKey");
        assert_eq!(provider.config.model, "gemini-2.5-pro");
        assert!(provider.is_official());
    }

    #[test]
    fn test_from_api_key_with_base_url() {
        let provider = GeminiProvider::from_api_key_with_base_url(
            "test-key".to_string(),
            "https://zenmux.ai/api".to_string(),
            "gemini-2.5-flash".to_string(),
        );
        assert_eq!(provider.config.auth.key, "test-key");
        assert_eq!(
            provider.config.auth.base_url,
            Some("https://zenmux.ai/api".to_string())
        );
        assert!(!provider.is_official());
    }

    #[test]
    fn test_build_url() {
        let provider =
            GeminiProvider::from_api_key("test-key".to_string(), "gemini-2.5-flash".to_string());
        assert_eq!(
            provider.build_url("gemini-2.5-flash", "generateContent"),
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent"
        );
    }

    #[test]
    fn test_build_url_custom_base() {
        let provider = GeminiProvider::from_api_key_with_base_url(
            "test-key".to_string(),
            "https://zenmux.ai/api".to_string(),
            "gemini-2.5-flash".to_string(),
        );
        assert_eq!(
            provider.build_url("gemini-2.5-flash", "generateContent"),
            "https://zenmux.ai/api/v1beta/models/gemini-2.5-flash:generateContent"
        );
    }
}
