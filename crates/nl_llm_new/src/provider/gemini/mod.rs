//! Gemini Provider 模块
//!
//! 通过 API Key 认证，调用 Google AI Studio 或第三方转发站

pub mod common;
pub mod compiler;
pub mod config;
pub mod provider;

// 重导出常用类型
pub use config::GeminiConfig;
pub use provider::{GeminiProvider, GoogleAIStudioProvider};
