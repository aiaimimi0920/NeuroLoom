use crate::model::{Capability, DefaultModelResolver, ModelResolver};

/// AICodeMirror 模型解析器
pub struct AiCodeMirrorModelResolver {
    inner: DefaultModelResolver,
}

impl AiCodeMirrorModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        inner.extend_aliases(vec![
            ("aicodemirror", "claude-sonnet-4-5-20250929"),
            ("sonnet", "claude-sonnet-4-5-20250929"),
            ("claude", "claude-sonnet-4-5-20250929"),
            ("sonnet-4.6", "claude-sonnet-4-6"),
            ("opus", "claude-opus-4-20250514"),
            ("opus-4.6", "claude-opus-4-6"),
            ("haiku", "claude-haiku-4-5-20251001"),
            ("haiku-4.5", "claude-haiku-4-5-20251001"),
            ("3.7", "claude-3-7-sonnet-20250219"),
            ("3.5", "claude-3-5-sonnet-20241022"),
        ]);

        let claude_caps = Capability::CHAT | Capability::TOOLS | Capability::STREAMING | Capability::VISION | Capability::THINKING;

        let all_models = [
            "claude-sonnet-4-6",
            "claude-opus-4-6",
            "claude-sonnet-4-5-20250929", "claude-haiku-4-5-20251001",
            "claude-opus-4-20250514", "claude-sonnet-4-20250514",
            "claude-3-7-sonnet-20250219", "claude-3-5-sonnet-20241022",
        ];

        inner.extend_capabilities(all_models.iter().map(|m| (*m, claude_caps)).collect());
        inner.extend_context_lengths(all_models.iter().map(|m| (*m, 200_000usize)).collect());

        Self { inner }
    }
}

impl Default for AiCodeMirrorModelResolver {
    fn default() -> Self { Self::new() }
}

impl ModelResolver for AiCodeMirrorModelResolver {
    fn resolve(&self, model: &str) -> String { self.inner.resolve(model) }
    fn has_capability(&self, model: &str, cap: Capability) -> bool { self.inner.has_capability(model, cap) }
    fn max_context(&self, model: &str) -> usize { self.inner.max_context(model) }
    fn context_window_hint(&self, model: &str) -> (usize, usize) { self.inner.context_window_hint(model) }
}
