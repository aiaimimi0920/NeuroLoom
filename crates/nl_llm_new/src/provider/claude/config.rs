use crate::auth::{ApiKeyConfig, ApiKeyProvider};
use std::collections::HashMap;
use std::path::PathBuf;

/// Claude Provider 配置
#[derive(Debug, Clone)]
pub struct ClaudeConfig {
    pub auth: ClaudeAuth,
    pub model: String,
    pub extra_headers: HashMap<String, String>,
}

/// Claude 认证方式
#[derive(Debug, Clone)]
pub enum ClaudeAuth {
    /// API Key 认证（官方或转发站）
    ApiKey(ApiKeyConfig),
    /// OAuth 认证
    OAuth { token_path: PathBuf },
}

impl ClaudeConfig {
    /// 快捷创建：从 API Key 字符串和模型名创建
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            auth: ClaudeAuth::ApiKey(ApiKeyConfig::new(api_key, ApiKeyProvider::Anthropic)),
            model,
            extra_headers: HashMap::new(),
        }
    }

    /// 从 API Key 配置创建（官方或转发站）
    pub fn with_api_key(api_key_cfg: ApiKeyConfig, model: String) -> Self {
        Self {
            auth: ClaudeAuth::ApiKey(api_key_cfg),
            model,
            extra_headers: HashMap::new(),
        }
    }

    /// 从 OAuth Token 文件创建
    pub fn with_oauth(token_path: PathBuf, model: String) -> Self {
        Self {
            auth: ClaudeAuth::OAuth { token_path },
            model,
            extra_headers: HashMap::new(),
        }
    }
}
