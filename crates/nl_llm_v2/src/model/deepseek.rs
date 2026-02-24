use super::default::DefaultModelResolver;
use super::resolver::{ModelResolver, Capability};

/// DeepSeek 模型解析器
///
/// 支持的模型（截至 2026-02-25）：
/// - deepseek-chat: 通用对话模型
/// - deepseek-coder: 代码专用模型
/// - deepseek-reasoner: 推理模型
pub struct DeepSeekModelResolver {
    inner: DefaultModelResolver,
}

impl DeepSeekModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // === 模型别名 ===
        inner.extend_aliases(vec![
            ("deepseek", "deepseek-chat"),
            ("ds", "deepseek-chat"),
            ("chat", "deepseek-chat"),
            ("coder", "deepseek-coder"),
            ("reasoner", "deepseek-reasoner"),
        ]);

        // === 能力配置 ===
        inner.extend_capabilities(vec![
            ("deepseek-chat", Capability::CHAT | Capability::TOOLS | Capability::STREAMING),
            ("deepseek-coder", Capability::CHAT | Capability::TOOLS | Capability::STREAMING),
            ("deepseek-reasoner", Capability::CHAT | Capability::STREAMING | Capability::THINKING),
        ]);

        // === 上下文长度 ===
        inner.extend_context_lengths(vec![
            ("deepseek-chat", 64_000),
            ("deepseek-coder", 64_000),
            ("deepseek-reasoner", 64_000),
        ]);

        Self { inner }
    }
}

impl Default for DeepSeekModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for DeepSeekModelResolver {
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
