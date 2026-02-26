use super::resolver::{Capability, Modality, ModelResolver};

/// Doubao 视频模型解析器
///
/// 从 `doubao-video` 等别名映射到底层的具体模型。
pub struct DoubaoModelResolver {}

impl DoubaoModelResolver {
    pub fn new() -> Self {
        Self {}
    }
}

impl ModelResolver for DoubaoModelResolver {
    fn resolve(&self, model: &str) -> String {
        match model.to_lowercase().as_str() {
            "doubao-video" | "seedance" | "seedance-pro" => {
                "doubao-seedance-1-0-pro-250528".to_string()
            }
            "seedance-lite" => "doubao-seedance-1-0-lite-t2v".to_string(),
            "seedance-1.5" => "doubao-seedance-1-5-pro-251215".to_string(),
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
        Some((4.0, Modality::ImageGeneration)) // Treated as visual generation
    }
}
