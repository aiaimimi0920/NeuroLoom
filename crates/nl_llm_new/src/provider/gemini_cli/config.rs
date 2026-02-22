use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiCliConfig {
    pub model: String,
    pub token_path: PathBuf,
}

impl Default for GeminiCliConfig {
    fn default() -> Self {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".to_string());
        let token_path = std::path::PathBuf::from(home)
            .join(".nl_llm")
            .join("gemini_cli_token.json");
            
        Self {
            model: "gemini-2.5-flash".to_string(),
            token_path,
        }
    }
}
