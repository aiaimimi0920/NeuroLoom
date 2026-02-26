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
        if model.is_empty() {
            "m3e-base".to_string()
        } else {
            model.to_string()
        }
    }

    fn has_capability(&self, model: &str, cap: Capability) -> bool {
        let resolved = self.resolve(model);

        // 如果明确是 MokaAI 的嵌入模型，则不允许 Chat/Tools，仅允许被当成 Embedding 调度
        if resolved.starts_with("m3e") {
            return cap.is_empty(); // Embedding models strictly do not have generation capabilities
        }

        // 默认让其他输入模型通过对讲功能，依赖源站返回情况
        cap.contains(Capability::CHAT) || cap.contains(Capability::STREAMING)
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
