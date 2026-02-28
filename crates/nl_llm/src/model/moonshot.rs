use super::default::DefaultModelResolver;
use super::resolver::{Capability, ModelResolver};

/// Moonshot (月之暗面) 模型解析器
///
/// 支持的模型（截至 2026-02-25）：
/// - moonshot-v1-8k: 8K 上下文
/// - moonshot-v1-32k: 32K 上下文
/// - moonshot-v1-128k: 128K 上下文
pub struct MoonshotModelResolver {
    inner: DefaultModelResolver,
}

impl MoonshotModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // === 模型别名 ===
        inner.extend_aliases(vec![
            ("moonshot", "moonshot-v1-8k"),
            ("moonshot-8k", "moonshot-v1-8k"),
            ("moonshot-32k", "moonshot-v1-32k"),
            ("moonshot-128k", "moonshot-v1-128k"),
        ]);

        // === 能力配置 ===
        inner.extend_capabilities(vec![
            ("moonshot-v1-8k", Capability::CHAT | Capability::STREAMING),
            ("moonshot-v1-32k", Capability::CHAT | Capability::STREAMING),
            ("moonshot-v1-128k", Capability::CHAT | Capability::STREAMING),
        ]);

        // === 上下文长度 ===
        inner.extend_context_lengths(vec![
            ("moonshot-v1-8k", 8_192),
            ("moonshot-v1-32k", 32_768),
            ("moonshot-v1-128k", 131_072),
        ]);

        Self { inner }
    }
}

impl Default for MoonshotModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for MoonshotModelResolver {
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
