use crate::model::{Capability, ModelResolver};

/// 适用于“MokaAI”渠道的模型解析器
pub struct MokaModelResolver;

impl MokaModelResolver {
    pub fn new() -> Self {
        Self
    }
}

impl ModelResolver for MokaModelResolver {
    fn resolve(&self, model: &str) -> String {
        // 如果未指定具体模型，默认给一个基础的 embedding 优先方案（Moka 主要是这个用处）
        // 但也可配置通用 fallback
        if model.trim().is_empty() {
            "m3e-base".to_string()
        } else {
            model.to_string()
        }
    }

    fn has_capability(&self, model: &str, cap: Capability) -> bool {
        let resolved = self.resolve(model);

        // 当前 Moka 预设主要覆盖 embedding 模型；非 embedding 模型保持保守的 Chat/Stream 能力。
        let supported = if resolved.starts_with("m3e") {
            Capability::empty()
        } else {
            Capability::CHAT | Capability::STREAMING
        };

        supported.contains(cap)
    }

    fn max_context(&self, _model: &str) -> usize {
        // 大多常见开源和 embedding 模型的上限，这里给宽泛些
        32_000
    }

    fn context_window_hint(&self, model: &str) -> (usize, usize) {
        let max = self.max_context(model);
        (max * 3 / 4, max / 4)
    }

    fn intelligence_and_modality(&self, model: &str) -> Option<(f32, crate::model::resolver::Modality)> {
        if model.starts_with("m3e") {
            Some((3.0, crate::model::resolver::Modality::Embedding))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_embedding_model_for_empty_input() {
        let resolver = MokaModelResolver::new();
        assert_eq!(resolver.resolve(""), "m3e-base");
        assert_eq!(resolver.resolve("   "), "m3e-base");
    }

    #[test]
    fn embedding_models_do_not_report_chat_capability() {
        let resolver = MokaModelResolver::new();
        assert!(!resolver.has_capability("m3e-base", Capability::CHAT));
        assert!(!resolver.has_capability("m3e-base", Capability::STREAMING));
    }

    #[test]
    fn non_embedding_models_are_conservatively_chat_streaming_only() {
        let resolver = MokaModelResolver::new();
        assert!(resolver.has_capability("custom-chat", Capability::CHAT));
        assert!(resolver.has_capability("custom-chat", Capability::STREAMING));
        assert!(!resolver.has_capability("custom-chat", Capability::TOOLS));
        assert!(!resolver.has_capability("custom-chat", Capability::CHAT | Capability::TOOLS));
    }
}
