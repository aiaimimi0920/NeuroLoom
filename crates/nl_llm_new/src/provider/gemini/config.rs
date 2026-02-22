use crate::auth::ApiKeyConfig;
use std::collections::HashMap;

/// Gemini Provider 配置 (API Key)
#[derive(Debug, Clone)]
pub struct GeminiConfig {
    pub auth: ApiKeyConfig,
    pub model: String,
    pub extra_headers: HashMap<String, String>,
}

impl GeminiConfig {
    pub fn new(auth: ApiKeyConfig, model: String) -> Self {
        Self {
            auth,
            model,
            extra_headers: HashMap::new(),
        }
    }
}
