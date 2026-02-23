//! Gemini Provider 模块
//!
//! 通过 API Key 认证，调用 Google AI Studio 或第三方转发站

pub mod common;
pub mod config;
pub mod provider;

// 重导出常用类型和函数
pub use common::{compile_request, parse_response, parse_sse_stream};
pub use config::GeminiConfig;
pub use provider::{GeminiProvider, GoogleAIStudioProvider};
