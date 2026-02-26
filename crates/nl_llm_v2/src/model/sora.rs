use super::resolver::{ModelResolver, Capability, Modality};

/// Sora 视频模型解析器
pub struct SoraModelResolver {}

impl SoraModelResolver {
    pub fn new() -> Self {
        Self {}
    }
}

impl ModelResolver for SoraModelResolver {
    fn resolve(&self, model: &str) -> String {
        match model.to_lowercase().as_str() {
            "sora" | "sora-2" | "sora_video" => "sora-2".to_string(),
            "sora-pro" | "sora-2-pro" => "sora-2-pro".to_string(),
            _ => model.to_string(),
        }
    }

    fn has_capability(&self, _model: &str, capability: Capability) -> bool {
        matches!(capability, Capability::VISION) // Video is treated as vision proxy in primitive
    }

    fn context_window_hint(&self, model: &str) -> (usize, usize) {
        let name = model.to_lowercase();
        if name.contains("pro") {
            (8192, 4096)
        } else {
            (4096, 4096)
        }
    }

    fn max_context(&self, model: &str) -> usize {
        let (max, _) = self.context_window_hint(model);
        max
    }

    fn intelligence_and_modality(&self, _model: &str) -> Option<(f32, Modality)> {
        Some((4.5, Modality::ImageGeneration)) // Treated as visual generation
    }
}
