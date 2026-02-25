use crate::model::{Capability, DefaultModelResolver, ModelResolver};

/// Cubence 模型解析器
///
/// ## 别名
///
/// | 别名 | 解析为 |
/// |------|--------|
/// | `cubence` / `sonnet` | `claude-sonnet-4-5-20250929` |
/// | `4o` | `gpt-4o` |
/// | `4o-mini` | `gpt-4o-mini` |
/// | `gemini` | `gemini-2.0-flash` |
/// | `gemini-pro` | `gemini-2.5-pro` |
pub struct CubenceModelResolver {
    inner: DefaultModelResolver,
}

impl CubenceModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        inner.extend_aliases(vec![
            ("cubence", "claude-sonnet-4-5-20250929"),
            ("sonnet", "claude-sonnet-4-5-20250929"),
            ("claude", "claude-sonnet-4-5-20250929"),
            ("4o", "gpt-4o"),
            ("4o-mini", "gpt-4o-mini"),
            ("gemini", "gemini-2.0-flash"),
            ("gemini-pro", "gemini-2.5-pro"),
        ]);

        let claude_caps = Capability::CHAT | Capability::TOOLS | Capability::STREAMING | Capability::VISION | Capability::THINKING;
        let gpt_caps = Capability::CHAT | Capability::TOOLS | Capability::STREAMING | Capability::VISION;
        let gemini_caps = Capability::CHAT | Capability::TOOLS | Capability::STREAMING | Capability::VISION | Capability::THINKING;

        inner.extend_capabilities(vec![
            ("claude-sonnet-4-5-20250929", claude_caps),
            ("claude-3-5-sonnet-20241022", claude_caps),
            ("gpt-4o", gpt_caps),
            ("gpt-4o-mini", gpt_caps),
            ("gemini-2.0-flash", gemini_caps),
            ("gemini-2.5-pro", gemini_caps),
        ]);

        inner.extend_context_lengths(vec![
            ("claude-sonnet-4-5-20250929", 200_000),
            ("claude-3-5-sonnet-20241022", 200_000),
            ("gpt-4o", 128_000),
            ("gpt-4o-mini", 128_000),
            ("gemini-2.0-flash", 1_000_000),
            ("gemini-2.5-pro", 1_000_000),
        ]);

        Self { inner }
    }
}

impl Default for CubenceModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for CubenceModelResolver {
    fn resolve(&self, model: &str) -> String { self.inner.resolve(model) }
    fn has_capability(&self, model: &str, cap: Capability) -> bool { self.inner.has_capability(model, cap) }
    fn max_context(&self, model: &str) -> usize { self.inner.max_context(model) }
    fn context_window_hint(&self, model: &str) -> (usize, usize) { self.inner.context_window_hint(model) }
}
