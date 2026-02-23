//! Vertex AI Provider 模块
//!
//! 专用于 Google Cloud Vertex AI，通过 Service Account JSON 认证

pub mod config;
pub mod provider;

// 重导出
pub use config::VertexConfig;
pub use provider::{VertexProvider, vertex_base_url, normalize_private_key};
