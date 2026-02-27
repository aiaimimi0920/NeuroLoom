use crate::model::{Capability, DefaultModelResolver, ModelResolver};

/// 无问芯穹（Infinigence AI）模型解析器
///
/// Infinigence 提供 OpenAI 兼容网关，模型更新较快。
/// 这里维护常用别名，避免业务侧散落硬编码模型名。
pub struct InfiniModelResolver {
    inner: DefaultModelResolver,
}

impl InfiniModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        inner.extend_aliases(vec![
            ("infini", "deepseek-v3.1"),
            ("deepseek-v3", "deepseek-v3.1"),
            ("v3", "deepseek-v3.1"),
            ("deepseek-r1", "deepseek-r1"),
            ("r1", "deepseek-r1"),
            ("qwen-coder", "qwen3-coder-plus"),
        ]);

        let chat_caps = Capability::CHAT | Capability::TOOLS | Capability::STREAMING;
        let think_caps = Capability::CHAT | Capability::STREAMING | Capability::THINKING;

        inner.extend_capabilities(vec![
            ("deepseek-v3.1", chat_caps),
            ("deepseek-r1", think_caps),
            ("qwen3-coder-plus", chat_caps | Capability::THINKING),
            ("qwen2.5-vl-72b-instruct", chat_caps | Capability::VISION),
        ]);

        inner.extend_context_lengths(vec![
            ("deepseek-v3.1", 128_000),
            ("deepseek-r1", 128_000),
            ("qwen3-coder-plus", 256_000),
            ("qwen2.5-vl-72b-instruct", 128_000),
        ]);

        Self { inner }
    }
}

impl Default for InfiniModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for InfiniModelResolver {
    fn resolve(&self, model: &str) -> String {
        self.inner.resolve(model)
    }

    fn has_capability(&self, model: &str, cap: Capability) -> bool {
        self.inner.has_capability(model, cap)
    }

    fn max_context(&self, model: &str) -> usize {
        self.inner.max_context(model)
    }

    fn context_window_hint(&self, model: &str) -> (usize, usize) {
        self.inner.context_window_hint(model)
    }

    fn intelligence_and_modality(
        &self,
        _model: &str,
    ) -> Option<(f32, crate::model::resolver::Modality)> {
        None
    }
}
