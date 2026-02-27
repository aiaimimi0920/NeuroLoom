use crate::model::{Capability, DefaultModelResolver, ModelResolver};

/// 无问芯穹 (Infinigence AI) 模型解析器
///
/// 无问芯穹使用 OpenAI 兼容接口，支持多个主流模型族的直连路由。
/// 该解析器提供常用别名，便于在业务中使用统一短名。
pub struct InfiniModelResolver {
    inner: DefaultModelResolver,
}

impl InfiniModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // 常用别名（面向默认可用的高频模型）
        inner.extend_aliases(vec![
            ("infini", "deepseek-v3"),
            ("xinqiong", "deepseek-v3"),
            ("ds", "deepseek-v3"),
            ("qwen", "qwen2.5-72b-instruct"),
            ("qwen3", "qwen3-32b"),
            ("glm", "glm-4.5"),
        ]);

        let chat_caps = Capability::CHAT | Capability::STREAMING;
        let tool_caps = chat_caps | Capability::TOOLS;

        inner.extend_capabilities(vec![
            ("deepseek-v3", tool_caps),
            ("qwen2.5-72b-instruct", tool_caps),
            ("qwen3-32b", tool_caps),
            ("glm-4.5", tool_caps),
            ("doubao-seed-1-6", chat_caps),
        ]);

        inner.extend_context_lengths(vec![
            ("deepseek-v3", 128_000),
            ("qwen2.5-72b-instruct", 128_000),
            ("qwen3-32b", 128_000),
            ("glm-4.5", 128_000),
            ("doubao-seed-1-6", 128_000),
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
