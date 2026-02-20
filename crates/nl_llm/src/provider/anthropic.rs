//! Anthropic Provider

use crate::prompt_ast::PromptAst;

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

    /// 将 Prompt AST 编译为 Anthropic 风格 payload（XML 封装）
    pub fn compile_request(&self, ast: &PromptAst) -> serde_json::Value {
        serde_json::json!({
            "model": self.config.model,
            "max_tokens": 4096,
            "input_format": "xml",
            "prompt": ast.to_anthropic_xml()
        })
    }

    pub async fn complete(&self, ast: &PromptAst) -> nl_core::Result<String> {
        let body = self.compile_request(ast);
        Ok(format!(
            "anthropic request prepared: {}",
            serde_json::to_string(&body).unwrap_or_default()
        ))
    }
}
