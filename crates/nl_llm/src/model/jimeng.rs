use super::resolver::{Capability, Modality, ModelResolver};

pub struct JimengModelResolver {}

impl JimengModelResolver {
    pub fn new() -> Self {
        Self {}
    }
}

impl ModelResolver for JimengModelResolver {
    fn resolve(&self, model: &str) -> String {
        match model {
            "jimeng-v3" | "jimeng-v3.0" | "jimeng_t2v_v30" => "jimeng_t2v_v30".to_string(),
            _ => model.to_string(),
        }
    }

    fn has_capability(&self, _model: &str, capability: Capability) -> bool {
        matches!(capability, Capability::VISION)
    }

    fn context_window_hint(&self, _model: &str) -> (usize, usize) {
        (0, 0)
    }

    fn max_context(&self, _model: &str) -> usize {
        0
    }

    fn intelligence_and_modality(&self, _model: &str) -> Option<(f32, Modality)> {
        Some((4.0, Modality::ImageGeneration))
    }
}
