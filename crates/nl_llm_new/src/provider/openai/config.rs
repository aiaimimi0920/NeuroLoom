use crate::auth::{ApiKeyConfig, ApiKeyProvider};
use std::collections::HashMap;

/// OpenAI Provider 配置
#[derive(Debug, Clone)]
pub struct OpenAIConfig {
    pub auth: ApiKeyConfig,
    pub model: String,
    pub extra_headers: HashMap<String, String>,
}

impl OpenAIConfig {
    /// 快捷创建：从 API Key 字符串和模型名创建
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            auth: ApiKeyConfig::new(api_key, ApiKeyProvider::OpenAI),
            model,
            extra_headers: HashMap::new(),
        }
    }

    /// 从 API Key 配置创建
    pub fn with_api_key(api_key_cfg: ApiKeyConfig, model: String) -> Self {
        Self {
            auth: api_key_cfg,
            model,
            extra_headers: HashMap::new(),
        }
    }
}
