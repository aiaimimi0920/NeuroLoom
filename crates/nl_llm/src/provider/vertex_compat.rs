//! Vertex Compat Provider 实现
//!
//! 支持第三方 Vertex 兼容服务（如 zenmux.ai）通过 API Key + Base URL 访问
//!
//! URL 格式: `{base_url}/v1/publishers/google/models/{model}:{action}`
//! 认证方式: `x-goog-api-key` header

use crate::prompt_ast::PromptAst;
use crate::provider::black_magic_proxy::BlackMagicProxySpec;
use crate::provider::gemini_common::{
    compile_gemini_request, parse_gemini_response, parse_gemini_sse_stream,
};
use serde::{Deserialize, Serialize};

// ── 常量 ────────────────────────────────────────────────────────────────────────
const VERTEX_COMPAT_API_VERSION: &str = "v1";

// ── 数据结构 ────────────────────────────────────────────────────────────────────
/// Vertex Compat provider 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexCompatConfig {
    /// API Key（由第三方服务提供）
    pub api_key: String,
    /// 第三方服务的 Base URL（必填，如 https://zenmux.ai/api）
    pub base_url: String,
    /// 模型，如 "gemini-2.5-flash"
    pub model: String,
}

pub struct VertexCompatProvider {
    config: VertexCompatConfig,
    client: reqwest::Client,
}

// ── 主实现 ───────────────────────────────────────────────────────────────────────
impl VertexCompatProvider {
    pub fn new(config: VertexCompatConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    /// 以 API key、base URL 和模型名称构建
    pub fn from_api_key_with_base_url(
        api_key: String,
        base_url: String,
        model: String,
    ) -> Self {
        Self::new(VertexCompatConfig {
            api_key,
            base_url,
            model,
        })
    }

    pub fn get_spec(&self) -> BlackMagicProxySpec {
        use crate::provider::black_magic_proxy::{
            BlackMagicProxyTarget, ProxyExposure, ProxyExposureKind,
        };
        BlackMagicProxySpec {
            target: BlackMagicProxyTarget::VertexCompat,
            default_base_url: self.config.base_url.clone(),
            exposures: vec![ProxyExposure {
                kind: ProxyExposureKind::Api,
                path: format!(
                    "/{}/publishers/google/models/{{model}}:generateContent",
                    VERTEX_COMPAT_API_VERSION
                ),
                method: "POST".to_string(),
                auth_header: Some("x-goog-api-key".to_string()),
                auth_prefix: Some("".to_string()),
                cli_command: None,
                cli_args: vec![],
                notes: "Vertex Compat (第三方转发站)".to_string(),
            }],
            notes: "Vertex Compat Provider (如 zenmux.ai)".to_string(),
        }
    }

    /// 构造 API URL
    fn build_url(&self, action: &str) -> String {
        format!(
            "{}/{}/publishers/google/models/{}:{}",
            self.config.base_url.trim_end_matches('/'),
            VERTEX_COMPAT_API_VERSION,
            self.config.model,
            action
        )
    }

    /// 非流式生成（generateContent）
    pub async fn complete(&self, ast: &PromptAst) -> crate::Result<String> {
        let url = self.build_url("generateContent");
        let body = compile_gemini_request(ast);

        let resp = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("x-goog-api-key", &self.config.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                crate::NeuroLoomError::LlmProvider(format!(
                    "vertex_compat: generateContent request failed: {}",
                    e
                ))
            })?;

        let status = resp.status();
        let raw_text = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(crate::NeuroLoomError::LlmProvider(format!(
                "vertex_compat: generateContent failed ({}): {}",
                status,
                raw_text.trim()
            )));
        }

        parse_gemini_response(&raw_text)
    }

    /// 流式生成（streamGenerateContent?alt=sse），返回拼接后的完整文本
    pub async fn stream_complete(&self, ast: &PromptAst) -> crate::Result<String> {
        let url = format!("{}?alt=sse", self.build_url("streamGenerateContent"));
        let body = compile_gemini_request(ast);

        let resp = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("x-goog-api-key", &self.config.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                crate::NeuroLoomError::LlmProvider(format!(
                    "vertex_compat: streamGenerateContent request failed: {}",
                    e
                ))
            })?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(crate::NeuroLoomError::LlmProvider(format!(
                "vertex_compat: streamGenerateContent failed ({}): {}",
                status,
                text.trim()
            )));
        }

        parse_gemini_sse_stream(resp).await
    }

    /// Token 计数（countTokens）
    pub async fn count_tokens(&self, ast: &PromptAst) -> crate::Result<u64> {
        let url = self.build_url("countTokens");
        let body = compile_gemini_request(ast);

        let resp = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("x-goog-api-key", &self.config.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                crate::NeuroLoomError::LlmProvider(format!(
                    "vertex_compat: countTokens request failed: {}",
                    e
                ))
            })?;

        let status = resp.status();
        let raw_text = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(crate::NeuroLoomError::LlmProvider(format!(
                "vertex_compat: countTokens failed ({}): {}",
                status,
                raw_text.trim()
            )));
        }

        let json: serde_json::Value = serde_json::from_str(&raw_text).map_err(|e| {
            crate::NeuroLoomError::LlmProvider(format!(
                "vertex_compat: countTokens decode failed: {}",
                e
            ))
        })?;
        Ok(json
            .get("totalTokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0))
    }
}

// ── 测试 ─────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompt_ast::PromptNode;

    #[test]
    fn test_build_url() {
        let config = VertexCompatConfig {
            api_key: "test-key".to_string(),
            base_url: "https://zenmux.ai/api".to_string(),
            model: "gemini-2.5-flash".to_string(),
        };
        let provider = VertexCompatProvider::new(config);
        assert_eq!(
            provider.build_url("generateContent"),
            "https://zenmux.ai/api/v1/publishers/google/models/gemini-2.5-flash:generateContent"
        );
        assert_eq!(
            provider.build_url("streamGenerateContent"),
            "https://zenmux.ai/api/v1/publishers/google/models/gemini-2.5-flash:streamGenerateContent"
        );
    }

    #[test]
    fn test_build_url_trailing_slash() {
        let config = VertexCompatConfig {
            api_key: "test-key".to_string(),
            base_url: "https://example.com/api/".to_string(), // 尾部斜杠
            model: "gemini-2.5-flash".to_string(),
        };
        let provider = VertexCompatProvider::new(config);
        // 应该正确移除尾部斜杠
        assert_eq!(
            provider.build_url("generateContent"),
            "https://example.com/api/v1/publishers/google/models/gemini-2.5-flash:generateContent"
        );
    }

    #[test]
    fn test_from_api_key_with_base_url() {
        let provider = VertexCompatProvider::from_api_key_with_base_url(
            "test-key".to_string(),
            "https://zenmux.ai/api".to_string(),
            "gemini-2.5-pro".to_string(),
        );
        assert_eq!(provider.config.api_key, "test-key");
        assert_eq!(provider.config.base_url, "https://zenmux.ai/api");
        assert_eq!(provider.config.model, "gemini-2.5-pro");
    }

    #[test]
    fn test_spec() {
        let config = VertexCompatConfig {
            api_key: "test".to_string(),
            base_url: "https://custom.api.com".to_string(),
            model: "gemini-2.5-flash".to_string(),
        };
        let provider = VertexCompatProvider::new(config);
        let spec = provider.get_spec();
        assert_eq!(spec.default_base_url, "https://custom.api.com");
        assert_eq!(spec.exposures.len(), 1);
        assert_eq!(
            spec.exposures[0].auth_header,
            Some("x-goog-api-key".to_string())
        );
    }

    #[test]
    fn test_request_body() {
        let config = VertexCompatConfig {
            api_key: "test-key".to_string(),
            base_url: "https://zenmux.ai/api".to_string(),
            model: "gemini-2.5-flash".to_string(),
        };
        let _provider = VertexCompatProvider::new(config);
        let ast = PromptAst::new()
            .push(PromptNode::System("Be helpful.".to_string()))
            .push(PromptNode::User("Hello!".to_string()));
        let body = compile_gemini_request(&ast);

        // systemInstruction 应该被提取
        assert!(body.get("systemInstruction").is_some());
        let contents = body["contents"].as_array().unwrap();
        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0]["role"], "user");
    }
}
