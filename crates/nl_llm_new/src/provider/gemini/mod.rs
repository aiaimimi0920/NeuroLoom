//! Gemini Provider 模块
//!
//! 通过 API Key 认证，调用 Google AI Studio 或第三方转发站。
//!
//! # 协议说明
//!
//! 本模块实现了 Gemini 原生协议和 CloudCode 方言壳。
//! 详见 [`protocol`] 模块的文档。

pub mod protocol;
pub mod config;
pub mod provider;

// 重导出常用类型和函数
pub use protocol::{compile_request, parse_response, parse_sse_stream, CloudCodeProtocol};
pub use config::GeminiConfig;
pub use provider::{GeminiProvider, GoogleAIStudioProvider};

