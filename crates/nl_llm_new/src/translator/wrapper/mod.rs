pub mod antigravity;
pub mod claude;
pub mod codex;
pub mod gemini;
pub mod gemini_cli;
pub mod openai;

use serde::{Deserialize, Serialize};

/// 包裹类型
///
/// 某些客户端会在请求中"包裹"额外的身份信息和工具定义
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum WrapperKind {
    #[default]
    None,
    ClaudeCode,
    GeminiCLI,
    Codex,
    Antigravity,
}

/// Gemini CLI 内置工具
pub const GEMINI_CLI_BUILTIN_TOOLS: &[&str] = &[
    "proxy_read_file",
    "proxy_write_file",
    "proxy_edit_file",
    "proxy_list_directory",
    "proxy_search_files",
    "proxy_execute_command",
    "proxy_create_directory",
    "proxy_delete_file",
    "proxy_move_file",
    "proxy_copy_file",
    "code_execution",
    "web_search",
];
