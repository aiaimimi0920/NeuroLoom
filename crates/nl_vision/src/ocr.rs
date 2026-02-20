//! OCR 模块

use serde::{Deserialize, Serialize};

/// OCR 结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrResult {
    /// 识别文本
    pub text: String,
    /// 置信度
    pub confidence: f64,
    /// 文本区域
    pub regions: Vec<TextRegion>,
}

/// 文本区域
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextRegion {
    pub text: String,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub confidence: f64,
}

/// OCR 引擎
pub struct OcrEngine {
    /// 是否启用
    enabled: bool,
}

impl OcrEngine {
    /// 创建新 OCR 引擎
    pub fn new() -> Self {
        Self { enabled: true }
    }

    /// 识别图像中的文本
    pub async fn recognize(&self, image: &[u8]) -> nl_core::Result<OcrResult> {
        // TODO: 实现实际的 OCR 逻辑
        Ok(OcrResult {
            text: String::new(),
            confidence: 0.0,
            regions: Vec::new(),
        })
    }

    /// 启用/禁用
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

impl Default for OcrEngine {
    fn default() -> Self {
        Self::new()
    }
}
