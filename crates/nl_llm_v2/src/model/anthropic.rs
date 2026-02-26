use super::default::DefaultModelResolver;
use super::resolver::{ModelResolver, Capability};

/// Anthropic Claude 模型解析器
///
/// 支持的模型（截至 2026-02-24）：
/// - Claude 4.6 系列：claude-opus-4-6, claude-sonnet-4-6
/// - Claude 4.5 系列：claude-opus-4-5-20251101, claude-sonnet-4-5-20250929, claude-haiku-4-5-20251001
/// - Claude 4 系列：claude-opus-4-20250514, claude-sonnet-4-20250514
/// - Claude 3.7：claude-3-7-sonnet-20250219
pub struct AnthropicModelResolver {
    inner: DefaultModelResolver,
}

impl AnthropicModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // === 模型别名 ===
        inner.extend_aliases(vec![
            // 便捷别名（指向最新版本）
            ("claude", "claude-sonnet-4-6"),
            ("claude-sonnet", "claude-sonnet-4-6"),
            ("claude-opus", "claude-opus-4-6"),
            ("claude-haiku", "claude-haiku-4-5-20251001"),
            // 带版本日期的别名（兼容旧代码）
            ("claude-sonnet-4", "claude-sonnet-4-20250514"),
            ("claude-opus-4", "claude-opus-4-20250514"),
            ("claude-3.7", "claude-3-7-sonnet-20250219"),
            // Claude 3.5 兼容别名（映射到对应版本）
            ("claude-3.5-sonnet", "claude-sonnet-4-20250514"),
            ("claude-3-5-sonnet", "claude-sonnet-4-20250514"),
        ]);

        // === 能力配置 ===
        // Claude 4.6 系列
        inner.extend_capabilities(vec![
            ("claude-opus-4-6", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING | Capability::THINKING),
            ("claude-sonnet-4-6", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING | Capability::THINKING),
            // Claude 4.5 系列
            ("claude-opus-4-5-20251101", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING | Capability::THINKING),
            ("claude-sonnet-4-5-20250929", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING | Capability::THINKING),
            ("claude-haiku-4-5-20251001", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING | Capability::THINKING),
            // Claude 4 系列
            ("claude-opus-4-20250514", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING | Capability::THINKING),
            ("claude-sonnet-4-20250514", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING | Capability::THINKING),
            // Claude 3.7
            ("claude-3-7-sonnet-20250219", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING | Capability::THINKING),
        ]);

        // === 上下文长度 ===
        inner.extend_context_lengths(vec![
            ("claude-opus-4-6", 1_000_000),        // 1M context
            ("claude-sonnet-4-6", 200_000),
            ("claude-opus-4-5-20251101", 200_000),
            ("claude-sonnet-4-5-20250929", 200_000),
            ("claude-haiku-4-5-20251001", 200_000),
            ("claude-opus-4-20250514", 200_000),
            ("claude-sonnet-4-20250514", 200_000),
            ("claude-3-7-sonnet-20250219", 200_000),
        ]);

        Self { inner }
    }
}

impl Default for AnthropicModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for AnthropicModelResolver {
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
