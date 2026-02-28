use crate::model::{Capability, DefaultModelResolver, ModelResolver};

/// PackyCode 模型解析器
///
/// ## 别名
///
/// | 别名 | 解析为 |
/// |------|--------|
/// | `packycode` / `4o-mini` | `gpt-4o-mini` |
/// | `4o` | `gpt-4o` |
/// | `4.1` | `gpt-4.1` |
/// | `sonnet` | `claude-sonnet-4-5-20250929` |
/// | `gemini` | `gemini-2.0-flash` |
/// | `deepseek` | `deepseek-chat` |
/// | `r1` | `deepseek-reasoner` |
pub struct PackyCodeModelResolver {
    inner: DefaultModelResolver,
}

impl PackyCodeModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        inner.extend_aliases(vec![
            ("packycode", "gpt-4o-mini"),
            ("4o", "gpt-4o"),
            ("4o-mini", "gpt-4o-mini"),
            ("4.1", "gpt-4.1"),
            ("4.1-mini", "gpt-4.1-mini"),
            ("sonnet", "claude-sonnet-4-5-20250929"),
            ("claude", "claude-sonnet-4-5-20250929"),
            ("gemini", "gemini-2.0-flash"),
            ("deepseek", "deepseek-chat"),
            ("r1", "deepseek-reasoner"),
        ]);

        let gpt_caps =
            Capability::CHAT | Capability::TOOLS | Capability::STREAMING | Capability::VISION;
        let claude_caps = gpt_caps | Capability::THINKING;
        let gemini_caps = gpt_caps | Capability::THINKING;
        let ds_chat_caps = Capability::CHAT | Capability::TOOLS | Capability::STREAMING;
        let ds_think_caps = Capability::CHAT | Capability::STREAMING | Capability::THINKING;

        inner.extend_capabilities(vec![
            ("gpt-4o", gpt_caps),
            ("gpt-4o-mini", gpt_caps),
            ("gpt-4.1", gpt_caps),
            ("gpt-4.1-mini", gpt_caps),
            ("claude-sonnet-4-5-20250929", claude_caps),
            ("claude-3-5-sonnet-20241022", claude_caps),
            ("gemini-2.0-flash", gemini_caps),
            ("deepseek-chat", ds_chat_caps),
            ("deepseek-reasoner", ds_think_caps),
        ]);

        inner.extend_context_lengths(vec![
            ("gpt-4o", 128_000),
            ("gpt-4o-mini", 128_000),
            ("gpt-4.1", 1_000_000),
            ("gpt-4.1-mini", 1_000_000),
            ("claude-sonnet-4-5-20250929", 200_000),
            ("claude-3-5-sonnet-20241022", 200_000),
            ("gemini-2.0-flash", 1_000_000),
            ("deepseek-chat", 64_000),
            ("deepseek-reasoner", 64_000),
        ]);

        Self { inner }
    }
}

impl Default for PackyCodeModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for PackyCodeModelResolver {
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
