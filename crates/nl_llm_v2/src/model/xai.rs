use super::default::DefaultModelResolver;
use super::resolver::{ModelResolver, Capability};

/// x.AI (Grok) 模型解析器
///
/// 支持的模型（参考 x.AI API 文档）：
/// - grok-4-latest
/// - grok-3-latest
/// - grok-3-mini
/// - grok-2-latest
/// - grok-vision-latest
pub struct XaiModelResolver {
    inner: DefaultModelResolver,
}

impl XaiModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // === 模型别名 ===
        inner.extend_aliases(vec![
            ("grok", "grok-4-latest"),
            ("grok-4", "grok-4-latest"),
            ("grok-3", "grok-3-latest"),
            ("grok-2", "grok-2-latest"),
            ("grok-vision", "grok-vision-latest"),
        ]);

        // === 能力配置 ===
        inner.extend_capabilities(vec![
            ("grok-4-latest", Capability::CHAT | Capability::STREAMING | Capability::TOOLS),
            ("grok-3-latest", Capability::CHAT | Capability::STREAMING | Capability::TOOLS),
            ("grok-3-mini", Capability::CHAT | Capability::STREAMING | Capability::TOOLS),
            ("grok-2-latest", Capability::CHAT | Capability::STREAMING | Capability::TOOLS),
            ("grok-vision-latest", Capability::CHAT | Capability::STREAMING | Capability::TOOLS | Capability::VISION),
        ]);

        // === 上下文长度 ===
        // Note: setting context lengths based on typical Grok specs (131k context window)
        inner.extend_context_lengths(vec![
            ("grok-4-latest", 131_072),
            ("grok-3-latest", 131_072),
            ("grok-3-mini", 131_072),
            ("grok-2-latest", 131_072),
            ("grok-vision-latest", 8_192),
        ]);

        Self { inner }
    }
}

impl Default for XaiModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for XaiModelResolver {
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
