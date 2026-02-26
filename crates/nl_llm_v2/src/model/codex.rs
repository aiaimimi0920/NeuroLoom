use crate::model::default::DefaultModelResolver;
use crate::model::resolver::{ModelResolver, Capability};

/// Codex 模型解析器
///
/// 模型列表参考 CLIProxyAPI 的模型定义
pub struct CodexModelResolver {
    inner: DefaultModelResolver,
}

impl CodexModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // 便捷别名
        inner.extend_aliases(vec![
            ("codex", "gpt-5.1-codex"),
            ("codex-mini", "gpt-5.1-codex-mini"),
            ("codex-max", "gpt-5.1-codex-max"),
            ("gpt5-codex", "gpt-5-codex"),
            ("gpt5-codex-mini", "gpt-5-codex-mini"),
        ]);

        // 能力配置
        inner.extend_capabilities(vec![
            ("gpt-5.1-codex-max", Capability::CHAT | Capability::TOOLS | Capability::STREAMING | Capability::THINKING),
            ("gpt-5.1-codex", Capability::CHAT | Capability::TOOLS | Capability::STREAMING | Capability::THINKING),
            ("gpt-5.1-codex-mini", Capability::CHAT | Capability::TOOLS | Capability::STREAMING | Capability::THINKING),
            ("gpt-5-codex", Capability::CHAT | Capability::TOOLS | Capability::STREAMING | Capability::THINKING),
            ("gpt-5-codex-mini", Capability::CHAT | Capability::TOOLS | Capability::STREAMING | Capability::THINKING),
        ]);

        // 上下文长度（所有模型 400K）
        inner.extend_context_lengths(vec![
            ("gpt-5.1-codex-max", 400_000),
            ("gpt-5.1-codex", 400_000),
            ("gpt-5.1-codex-mini", 400_000),
            ("gpt-5-codex", 400_000),
            ("gpt-5-codex-mini", 400_000),
        ]);

        Self { inner }
    }
}

impl Default for CodexModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for CodexModelResolver {
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
    fn intelligence_and_modality(&self, _model: &str) -> Option<(f32, crate::model::resolver::Modality)> { None }

}
