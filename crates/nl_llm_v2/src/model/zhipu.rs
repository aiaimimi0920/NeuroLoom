use super::default::DefaultModelResolver;
use super::resolver::{ModelResolver, Capability};

/// 智谱 AI (Zhipu) 模型解析器
///
/// 支持的模型（截至 2026-02-25）：
/// - glm-4: 旗舰模型
/// - glm-4-flash: 快速模型
/// - glm-4-plus: 增强模型
/// - glm-4-air: 轻量模型
pub struct ZhipuModelResolver {
    inner: DefaultModelResolver,
}

impl ZhipuModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // === 模型别名 ===
        inner.extend_aliases(vec![
            ("glm", "glm-4"),
            ("glm4", "glm-4"),
            ("glm-4", "glm-4"),
            ("glm-flash", "glm-4-flash"),
            ("glm-plus", "glm-4-plus"),
            ("glm-air", "glm-4-air"),
        ]);

        // === 能力配置 ===
        inner.extend_capabilities(vec![
            ("glm-4", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING),
            ("glm-4-flash", Capability::CHAT | Capability::TOOLS | Capability::STREAMING),
            ("glm-4-plus", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING),
            ("glm-4-air", Capability::CHAT | Capability::STREAMING),
        ]);

        // === 上下文长度 ===
        inner.extend_context_lengths(vec![
            ("glm-4", 128_000),
            ("glm-4-flash", 128_000),
            ("glm-4-plus", 128_000),
            ("glm-4-air", 128_000),
        ]);

        Self { inner }
    }
}

impl Default for ZhipuModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for ZhipuModelResolver {
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
}
