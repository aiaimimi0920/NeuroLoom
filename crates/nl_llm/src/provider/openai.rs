//! OpenAI Provider

use crate::prompt_ast::PromptAst;

/// OpenAI 配置
#[derive(Debug, Clone)]
pub struct OpenAIConfig {
    pub api_key: String,
    pub model: String,
    pub base_url: String,
}

impl Default for OpenAIConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: "gpt-4".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }
}

/// OpenAI Provider
pub struct OpenAIProvider {
    config: OpenAIConfig,
}

impl OpenAIProvider {
    pub fn new(config: OpenAIConfig) -> Self {
        Self { config }
    }

    pub fn default_provider() -> Self {
        Self::new(OpenAIConfig::default())
    }

    /// 将 Prompt AST 编译为 OpenAI 兼容请求体
    pub fn compile_request(&self, ast: &PromptAst) -> serde_json::Value {
        serde_json::json!({
            "model": self.config.model,
            "messages": ast.to_openai_messages(),
            "temperature": 0.2
        })
    }

    pub async fn complete(&self, ast: &PromptAst) -> nl_core::Result<String> {
        let body = self.compile_request(ast);
        Ok(format!(
            "openai request prepared: {}",
            serde_json::to_string(&body).unwrap_or_default()
        ))
    }
}
