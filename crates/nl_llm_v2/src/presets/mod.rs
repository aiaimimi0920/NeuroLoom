pub mod registry;

// 各预设平台独立模块
pub mod openai;
pub mod claude;
pub mod gemini;
pub mod vertex;
pub mod deepseek;
pub mod moonshot;
pub mod zhipu;
pub mod iflow;
pub mod openrouter;
pub mod gemini_cli;
pub mod vertex_api;
pub mod qwen;
pub mod kimi;
pub mod claude_oauth;
pub mod codex_oauth;
pub mod codex_api;
pub mod antigravity;
pub mod amp;
pub mod zai;

pub use registry::{PresetRegistry, REGISTRY};
