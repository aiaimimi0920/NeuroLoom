use super::default::DefaultModelResolver;
use super::resolver::{ModelResolver, Capability};

/// Kimi (Moonshot AI) 模型解析器
///
/// 支持的模型来自 CLIProxyAPI 参考：
/// - kimi-k2: 通用对话模型
/// - kimi-k2-thinking: 推理增强模型
/// - kimi-k2.5: 最新版本
pub struct KimiModelResolver {
    inner: DefaultModelResolver,
}

impl KimiModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // === 模型别名 ===
        inner.extend_aliases(vec![
            ("kimi", "k2"),
            ("kimi-thinking", "k2-thinking"),
            ("moonshot", "k2"),
            ("kimi-k2", "k2"),             // 兼容带有 kimi- 前缀的别名
            ("kimi-k2-thinking", "k2-thinking"),
            ("kimi-k2.5", "k2.5"),
        ]);

        // === 能力配置 ===
        inner.extend_capabilities(vec![
            ("k2", Capability::CHAT | Capability::TOOLS | Capability::STREAMING),
            ("k2-thinking", Capability::CHAT | Capability::TOOLS | Capability::STREAMING | Capability::THINKING),
            ("k2.5", Capability::CHAT | Capability::TOOLS | Capability::STREAMING | Capability::THINKING),
        ]);

        // === 上下文长度 ===
        inner.extend_context_lengths(vec![
            ("k2", 131_072),
            ("k2-thinking", 131_072),
            ("k2.5", 131_072),
        ]);

        Self { inner }
    }
}

impl Default for KimiModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for KimiModelResolver {
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
