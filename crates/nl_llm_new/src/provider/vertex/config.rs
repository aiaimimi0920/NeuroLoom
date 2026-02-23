//! Vertex AI Provider 配置
//!
//! 专用于 Google Cloud Vertex AI，通过 Service Account JSON 认证

use serde::{Deserialize, Serialize};

/// Vertex Provider 配置（Service Account 认证）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexConfig {
    /// 服务账号 JSON 字符串（完整内容）
    pub credentials_json: String,
    /// 区域，默认 "us-central1"
    pub location: Option<String>,
    /// 模型，如 "gemini-2.5-flash"
    pub model: String,
    /// 自定义 base URL（可选，覆盖默认的 aiplatform.googleapis.com）
    pub base_url: Option<String>,
}

impl VertexConfig {
    /// 创建新配置
    pub fn new(
        credentials_json: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            credentials_json: credentials_json.into(),
            location: None,
            model: model.into(),
            base_url: None,
        }
    }

    /// 设置区域
    pub fn with_location(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }

    /// 设置自定义 Base URL
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    /// 从文件加载
    pub fn from_file(
        path: &std::path::Path,
        model: impl Into<String>,
    ) -> std::io::Result<Self> {
        let credentials_json = std::fs::read_to_string(path)?;
        Ok(Self::new(credentials_json, model))
    }
}
