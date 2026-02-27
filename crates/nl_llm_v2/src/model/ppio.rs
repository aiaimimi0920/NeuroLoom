use crate::model::{Capability, DefaultModelResolver, ModelResolver};

/// PPIO 模型解析器
///
/// PPIO 提供 OpenAI 兼容网关，常见社区模型名称形如 `org/model/tier`。
/// 该解析器提供常用别名、能力和上下文窗口提示，避免调用方散落硬编码。
pub struct PpioModelResolver {
    inner: DefaultModelResolver,
}

impl PpioModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        inner.extend_aliases(vec![
            ("ppio", "deepseek/deepseek-v3/community"),
            ("deepseek-v3", "deepseek/deepseek-v3/community"),
            ("v3", "deepseek/deepseek-v3/community"),
            ("deepseek-r1", "deepseek/deepseek-r1/community"),
            ("r1", "deepseek/deepseek-r1/community"),
            (
                "qwen-coder",
                "qwen/qwen3-coder-480b-a35b-instruct/community",
            ),
        ]);

        let chat_caps = Capability::CHAT | Capability::TOOLS | Capability::STREAMING;
        let think_caps = Capability::CHAT | Capability::STREAMING | Capability::THINKING;

        inner.extend_capabilities(vec![
            ("deepseek/deepseek-v3/community", chat_caps),
            ("deepseek/deepseek-r1/community", think_caps),
            (
                "qwen/qwen3-coder-480b-a35b-instruct/community",
                chat_caps | Capability::THINKING,
            ),
        ]);

        inner.extend_context_lengths(vec![
            ("deepseek/deepseek-v3/community", 64_000),
            ("deepseek/deepseek-r1/community", 64_000),
            ("qwen/qwen3-coder-480b-a35b-instruct/community", 262_144),
        ]);

        Self { inner }
    }
}

impl Default for PpioModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for PpioModelResolver {
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
