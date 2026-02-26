use super::default::DefaultModelResolver;
use super::resolver::{Capability, Modality, ModelResolver};

/// Vidu 模型解析器
///
/// 参考（以 new-api 的 Vidu 适配器为准）：
/// - viduq2
/// - viduq1
/// - vidu2.0
/// - vidu1.5
pub struct ViduModelResolver {
    inner: DefaultModelResolver,
}

impl ViduModelResolver {
    pub fn new() -> Self {
        Self {
            inner: DefaultModelResolver::new(),
        }
    }
}

impl Default for ViduModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for ViduModelResolver {
    fn resolve(&self, model: &str) -> String {
        let m = model.trim();
        if m.is_empty() {
            return "viduq1".to_string();
        }

        match m.to_lowercase().as_str() {
            "vidu" | "q1" | "viduq1" => "viduq1".to_string(),
            "q2" | "viduq2" => "viduq2".to_string(),
            "2" | "2.0" | "vidu2" | "vidu2.0" => "vidu2.0".to_string(),
            "1.5" | "vidu1.5" => "vidu1.5".to_string(),
            _ => m.to_string(),
        }
    }

    fn has_capability(&self, _model: &str, cap: Capability) -> bool {
        match cap {
            // Vidu 是视频生成类平台：我们用 VISION 作为“视觉生成”占位能力，避免被当作纯文本聊天模型。
            Capability::VISION => true,
            _ => false,
        }
    }

    fn max_context(&self, _model: &str) -> usize {
        0
    }

    fn context_window_hint(&self, _model: &str) -> (usize, usize) {
        (0, 0)
    }

    fn intelligence_and_modality(&self, _model: &str) -> Option<(f32, Modality)> {
        Some((4.0, Modality::ImageGeneration))
    }
}
