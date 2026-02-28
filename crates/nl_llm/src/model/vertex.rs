use super::default::DefaultModelResolver;
use super::resolver::{Capability, ModelResolver};

/// Vertex AI 模型解析器
///
/// Vertex AI 运行 Gemini 系列模型，别名和能力配置与 GeminiModelResolver 一致。
pub struct VertexModelResolver {
    inner: DefaultModelResolver,
}

impl VertexModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // === 模型别名 ===
        inner.extend_aliases(vec![
            ("flash", "gemini-2.5-flash"),
            ("pro", "gemini-2.5-pro"),
            ("gemini-pro", "gemini-2.5-pro"),
            ("gemini-flash", "gemini-2.5-flash"),
            ("gemini-2-pro", "gemini-2.0-pro-exp-02-05"),
            ("gemini-2-flash", "gemini-2.0-flash"),
        ]);

        // === 能力配置 ===
        inner.extend_capabilities(vec![
            (
                "gemini-2.5-pro",
                Capability::CHAT
                    | Capability::VISION
                    | Capability::TOOLS
                    | Capability::STREAMING
                    | Capability::THINKING
                    | Capability::CODE_EXECUTION,
            ),
            (
                "gemini-2.5-flash",
                Capability::CHAT
                    | Capability::VISION
                    | Capability::TOOLS
                    | Capability::STREAMING
                    | Capability::THINKING,
            ),
            (
                "gemini-2.0-flash",
                Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING,
            ),
            (
                "gemini-2.0-pro-exp-02-05",
                Capability::CHAT
                    | Capability::VISION
                    | Capability::TOOLS
                    | Capability::STREAMING
                    | Capability::THINKING,
            ),
            (
                "gemini-1.5-pro",
                Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING,
            ),
            (
                "gemini-1.5-flash",
                Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING,
            ),
            (
                "gemini-1.5-pro-002",
                Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING,
            ),
            (
                "gemini-1.5-flash-002",
                Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING,
            ),
        ]);

        // === 上下文长度 ===
        inner.extend_context_lengths(vec![
            ("gemini-2.5-pro", 1_000_000),
            ("gemini-2.5-flash", 1_000_000),
            ("gemini-2.0-flash", 1_000_000),
            ("gemini-2.0-pro-exp-02-05", 1_000_000),
            ("gemini-1.5-pro", 2_000_000),
            ("gemini-1.5-flash", 1_000_000),
            ("gemini-1.5-pro-002", 2_000_000),
            ("gemini-1.5-flash-002", 1_000_000),
        ]);

        Self { inner }
    }
}

impl Default for VertexModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for VertexModelResolver {
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
