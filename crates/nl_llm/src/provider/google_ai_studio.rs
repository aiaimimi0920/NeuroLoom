//! Google AI Studio Provider 实现
//!
//! 通过 API Key 访问 Google AI Studio (generativelanguage.googleapis.com)
//!
//! URL 格式: `https://generativelanguage.googleapis.com/v1beta/models/{model}:{action}`
//! 认证方式: `x-goog-api-key` header

use crate::prompt_ast::PromptAst;
use crate::provider::black_magic_proxy::BlackMagicProxySpec;
use crate::provider::gemini_common::{
    compile_gemini_request, parse_gemini_response, parse_gemini_sse_stream,
};
use serde::{Deserialize, Serialize};

// ── 常量 ────────────────────────────────────────────────────────────────────────
const GOOGLE_AI_STUDIO_BASE_URL: &str = "https://generativelanguage.googleapis.com";
const GOOGLE_AI_STUDIO_API_VERSION: &str = "v1beta";

// ── 数据结构 ────────────────────────────────────────────────────────────────────
/// Google AI Studio provider 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleAIStudioConfig {
    /// Google API Key（从 https://aistudio.google.com/app/apikey 获取）
    pub api_key: String,
    /// 模型，如 "gemini-2.5-flash", "gemini-2.5-pro"
    pub model: String,
}

pub struct GoogleAIStudioProvider {
    config: GoogleAIStudioConfig,
    client: reqwest::Client,
}

// ── 主实现 ───────────────────────────────────────────────────────────────────────
impl GoogleAIStudioProvider {
    pub fn new(config: GoogleAIStudioConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    /// 以 API key 和模型名称构建
    pub fn from_api_key(api_key: String, model: String) -> Self {
        Self::new(GoogleAIStudioConfig { api_key, model })
    }

    pub fn get_spec(&self) -> BlackMagicProxySpec {
        use crate::provider::black_magic_proxy::{
            BlackMagicProxyTarget, ProxyExposure, ProxyExposureKind,
        };
        BlackMagicProxySpec {
            target: BlackMagicProxyTarget::GoogleAIStudio,
            default_base_url: GOOGLE_AI_STUDIO_BASE_URL.to_string(),
            exposures: vec![ProxyExposure {
                kind: ProxyExposureKind::Api,
                path: format!(
                    "/{}/models/{{model}}:generateContent",
                    GOOGLE_AI_STUDIO_API_VERSION
                ),
                method: "POST".to_string(),
                auth_header: Some("x-goog-api-key".to_string()),
                auth_prefix: Some("".to_string()),
                cli_command: None,
                cli_args: vec![],
                notes: "Google AI Studio Gemini API".to_string(),
            }],
            notes: "Google AI Studio (generativelanguage.googleapis.com)".to_string(),
        }
    }

    /// 构造 API URL
    fn build_url(&self, action: &str) -> String {
        format!(
            "{}/{}/models/{}:{}",
            GOOGLE_AI_STUDIO_BASE_URL,
            GOOGLE_AI_STUDIO_API_VERSION,
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
                    "google_ai_studio: generateContent request failed: {}",
                    e
                ))
            })?;

        let status = resp.status();
        let raw_text = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(crate::NeuroLoomError::LlmProvider(format!(
                "google_ai_studio: generateContent failed ({}): {}",
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
                    "google_ai_studio: streamGenerateContent request failed: {}",
                    e
                ))
            })?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(crate::NeuroLoomError::LlmProvider(format!(
                "google_ai_studio: streamGenerateContent failed ({}): {}",
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
                    "google_ai_studio: countTokens request failed: {}",
                    e
                ))
            })?;

        let status = resp.status();
        let raw_text = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(crate::NeuroLoomError::LlmProvider(format!(
                "google_ai_studio: countTokens failed ({}): {}",
                status,
                raw_text.trim()
            )));
        }

        let json: serde_json::Value = serde_json::from_str(&raw_text).map_err(|e| {
            crate::NeuroLoomError::LlmProvider(format!(
                "google_ai_studio: countTokens decode failed: {}",
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
        let config = GoogleAIStudioConfig {
            api_key: "test-key".to_string(),
            model: "gemini-2.5-flash".to_string(),
        };
        let provider = GoogleAIStudioProvider::new(config);
        assert_eq!(
            provider.build_url("generateContent"),
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent"
        );
        assert_eq!(
            provider.build_url("streamGenerateContent"),
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:streamGenerateContent"
        );
    }

    #[test]
    fn test_from_api_key() {
        let provider = GoogleAIStudioProvider::from_api_key(
            "AIzaSyTestKey".to_string(),
            "gemini-2.5-pro".to_string(),
        );
        assert_eq!(provider.config.api_key, "AIzaSyTestKey");
        assert_eq!(provider.config.model, "gemini-2.5-pro");
    }

    #[test]
    fn test_spec() {
        let config = GoogleAIStudioConfig {
            api_key: "test".to_string(),
            model: "gemini-2.5-flash".to_string(),
        };
        let provider = GoogleAIStudioProvider::new(config);
        let spec = provider.get_spec();
        assert_eq!(
            spec.default_base_url,
            "https://generativelanguage.googleapis.com"
        );
        assert_eq!(spec.exposures.len(), 1);
        assert_eq!(
            spec.exposures[0].auth_header,
            Some("x-goog-api-key".to_string())
        );
    }

    #[test]
    fn test_request_body() {
        let config = GoogleAIStudioConfig {
            api_key: "test-key".to_string(),
            model: "gemini-2.5-flash".to_string(),
        };
        let _provider = GoogleAIStudioProvider::new(config);
        let ast = PromptAst::new().push(PromptNode::User("Hello!".to_string()));
        let body = compile_gemini_request(&ast);

        let contents = body["contents"].as_array().unwrap();
        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0]["role"], "user");
        assert_eq!(contents[0]["parts"][0]["text"], "Hello!");
    }
}
