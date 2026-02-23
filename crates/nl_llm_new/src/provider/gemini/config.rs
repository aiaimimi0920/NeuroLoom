//! Gemini Provider 配置
//!
//! 支持 API Key 认证（官方/转发站）

use crate::auth::{ApiKeyConfig, ApiKeyProvider};
use std::collections::HashMap;

/// Gemini Provider 配置 (API Key 认证)
///
/// 设计说明：
/// - API Key 本身不区分官方/转发站
/// - 区分的关键是 `base_url` 配置
/// - None: 使用 Google AI Studio 官方端点
/// - Some(url): 使用转发站/代理
#[derive(Debug, Clone)]
pub struct GeminiConfig {
    /// API Key 配置
    pub auth: ApiKeyConfig,
    /// 默认模型
    pub model: String,
    /// 额外请求头
    pub extra_headers: HashMap<String, String>,
}

impl GeminiConfig {
    /// 创建新的配置
    pub fn new(auth: ApiKeyConfig, model: String) -> Self {
        Self {
            auth,
            model,
            extra_headers: HashMap::new(),
        }
    }

    /// 使用 API Key 创建配置（官方端点）
    pub fn with_api_key(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self::new(
            ApiKeyConfig::new(api_key, ApiKeyProvider::GeminiAIStudio),
            model.into(),
        )
    }

    /// 使用 API Key 和自定义 Base URL 创建配置（转发站）
    pub fn with_api_key_and_base_url(
        api_key: impl Into<String>,
        base_url: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self::new(
            ApiKeyConfig::new(api_key, ApiKeyProvider::GeminiAIStudio).with_base_url(base_url),
            model.into(),
        )
    }

    /// 是否为官方端点
    pub fn is_official(&self) -> bool {
        self.auth.is_official()
    }

    /// 获取 Base URL
    pub fn base_url(&self) -> String {
        self.auth
            .base_url
            .clone()
            .unwrap_or_else(|| ApiKeyProvider::GeminiAIStudio.default_base_url().to_string())
    }
}

impl Default for GeminiConfig {
    fn default() -> Self {
        Self::with_api_key("", "gemini-2.5-flash")
    }
}
