use super::resolver::{Capability, Modality, ModelResolver};

/// Vidu 模型解析器（v0：静态 alias + 兜底透传）
pub struct ViduModelResolver;

impl ViduModelResolver {
    pub fn new() -> Self {
        Self
    }
}

impl ModelResolver for ViduModelResolver {
    fn resolve(&self, model: &str) -> String {
        match model.to_lowercase().as_str() {
            // 常见别名
            "q2" | "viduq2" => "viduq2".to_string(),
            "q1" | "viduq1" => "viduq1".to_string(),
            "2" | "2.0" | "vidu2" | "vidu2.0" => "vidu2.0".to_string(),
            "1.5" | "vidu1.5" => "vidu1.5".to_string(),
            _ => model.to_string(),
        }
    }

    fn has_capability(&self, _model: &str, capability: Capability) -> bool {
        match capability {
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
