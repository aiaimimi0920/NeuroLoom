//! 格式定义

use serde::{Deserialize, Serialize};

/// 支持的格式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Format {
    /// OpenAI Chat Completions API
    #[default]
    OpenAI,
    /// OpenAI Responses API (Codex)
    OpenAIResponse,
    /// Anthropic Messages API
    Claude,
    /// Google Gemini API
    Gemini,
    /// Gemini CLI (Cloud Code Assist)
    GeminiCLI,
    /// OpenAI Codex
    Codex,
    /// Antigravity
    Antigravity,
}

impl Format {
    /// 从字符串解析
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "openai" => Some(Self::OpenAI),
            "openai-response" => Some(Self::OpenAIResponse),
            "claude" => Some(Self::Claude),
            "gemini" => Some(Self::Gemini),
            "gemini-cli" => Some(Self::GeminiCLI),
            "codex" => Some(Self::Codex),
            "antigravity" => Some(Self::Antigravity),
            _ => None,
        }
    }

    /// 转换为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            Format::OpenAI => "openai",
            Format::OpenAIResponse => "openai-response",
            Format::Claude => "claude",
            Format::Gemini => "gemini",
            Format::GeminiCLI => "gemini-cli",
            Format::Codex => "codex",
            Format::Antigravity => "antigravity",
        }
    }
}

impl std::fmt::Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
