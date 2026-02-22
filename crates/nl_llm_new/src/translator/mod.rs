//! 转换层
//!
//! 实现"解包 → 原语 → 封装"的转换流程

mod format;
mod pipeline;
mod detector;
mod error;

pub mod unwrapper;
pub mod wrapper;

pub use format::*;
pub use wrapper::WrapperKind;
pub use pipeline::*;
pub use detector::*;
pub use error::*;

/// Claude Code 内置工具列表（用于解包时过滤）
pub const CLAUDE_CODE_BUILTIN_TOOLS: &[&str] = &[
    "Bash",
    "Computer",
    "Edit",
    "Read",
    "Write",
    "Glob",
    "Grep",
    "LS",
    "TodoRead",
    "TodoWrite",
    "MultiEdit",
    "NotebookRead",
    "NotebookEdit",
    "WebFetch",
    "WebSearch",
];

