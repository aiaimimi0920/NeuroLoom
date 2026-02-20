//! Ollama Provider (本地模型)

use serde::{Deserialize, Serialize};

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

    pub async fn complete(&self, prompt: &str) -> nl_core::Result<String> {
        // TODO: 实现 Ollama API 调用
        Ok("Ollama response placeholder".to_string())
    }
}
