//! Ollama Provider (本地模型)

use crate::prompt_ast::PromptAst;

/// Ollama 配置
#[derive(Debug, Clone)]
pub struct OllamaConfig {
    pub base_url: String,
    pub model: String,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:11434".to_string(),
            model: "llama3".to_string(),
        }
    }
}

/// Ollama Provider
pub struct OllamaProvider {
    config: OllamaConfig,
}

impl OllamaProvider {
    pub fn new(config: OllamaConfig) -> Self {
        Self { config }
    }

    pub fn default_provider() -> Self {
        Self::new(OllamaConfig::default())
    }

    /// 将 Prompt AST 编译为 Ollama 友好的 ChatML 请求
    pub fn compile_request(&self, ast: &PromptAst) -> serde_json::Value {
        serde_json::json!({
            "model": self.config.model,
            "prompt": ast.to_chatml(),
            "stream": false
        })
    }

    pub async fn complete(&self, ast: &PromptAst) -> nl_core::Result<String> {
        let body = self.compile_request(ast);
        Ok(format!(
            "ollama request prepared: {}",
            serde_json::to_string(&body).unwrap_or_default()
        ))
    }
}
