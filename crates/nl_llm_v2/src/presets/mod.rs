pub mod registry;

// 各预设平台独立模块
pub mod openai;
pub mod anthropic;
pub mod gemini;
pub mod vertex;
pub mod deepseek;
pub mod moonshot;
pub mod zhipu;
pub mod iflow;
pub mod openrouter;
pub mod gemini_cli;
pub mod vertex_api;
pub mod antigravity;

pub use registry::{PresetRegistry, REGISTRY};
