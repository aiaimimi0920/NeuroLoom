//! Anthropic Provider

use serde::{Deserialize, Serialize};

/// Anthropic 配置
#[derive(Debug, Clone)]
pub struct AnthropicConfig {
    pub api_key: String,
    pub model: String,
    pub base_url: String,
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: "claude-3-5-sonnet".to_string(),
            base_url: "https://api.anthropic.com/v1".to_string(),
        }
    }
}

/// Anthropic Provider
pub struct AnthropicProvider {
    config: AnthropicConfig,
}

impl AnthropicProvider {
    pub fn new(config: AnthropicConfig) -> Self {
        Self { config }
    }

    pub fn default_provider() -> Self {
        Self::new(AnthropicConfig::default())
    }

    pub async fn complete(&self, prompt: &str) -> nl_core::Result<String> {
        // TODO: 实现 Anthropic API 调用
        Ok("Anthropic response placeholder".to_string())
    }
}
