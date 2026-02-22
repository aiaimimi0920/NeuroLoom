//! Antigravity Provider 配置

use std::path::PathBuf;

/// Antigravity Provider 配置
#[derive(Debug, Clone)]
pub struct AntigravityConfig {
    /// Token 文件路径
    pub token_path: PathBuf,
    /// 默认模型
    pub model: String,
}

impl AntigravityConfig {
    /// 创建新配置
    pub fn new(token_path: PathBuf, model: String) -> Self {
        Self { token_path, model }
    }

    /// 使用默认路径创建配置
    pub fn with_default_path(model: String) -> Self {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".to_string());
        let token_path = PathBuf::from(home)
            .join(".nl_llm")
            .join("antigravity_token.json");
        Self { token_path, model }
    }
}

impl Default for AntigravityConfig {
    fn default() -> Self {
        Self::with_default_path("gemini-2.5-flash".to_string())
    }
}
