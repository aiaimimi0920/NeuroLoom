use super::resolver::{ModelResolver, Capability, Modality};

/// Replicate 模型解析器
pub struct ReplicateModelResolver {}

impl ReplicateModelResolver {
    pub fn new() -> Self {
        Self {}
    }
}

impl ModelResolver for ReplicateModelResolver {
    fn resolve(&self, model: &str) -> String {
        // Just return the model. Replicate uses explicit <owner>/<name> or direct name
        model.to_string()
    }

    fn has_capability(&self, _model: &str, capability: Capability) -> bool {
        matches!(capability, Capability::VISION) // Video is treated as vision proxy in primitive
    }

    fn context_window_hint(&self, _model: &str) -> (usize, usize) {
        (4096, 4096)
    }

    fn max_context(&self, model: &str) -> usize {
        let (max, _) = self.context_window_hint(model);
        max
    }

    fn intelligence_and_modality(&self, _model: &str) -> Option<(f32, Modality)> {
        Some((4.0, Modality::ImageGeneration)) // Treated as visual generation
    }
}
