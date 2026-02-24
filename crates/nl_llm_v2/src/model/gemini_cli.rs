use super::default::DefaultModelResolver;
use super::resolver::{ModelResolver, Capability};

/// Gemini CLI 专属模型解析器
///
/// 基于 DefaultModelResolver 扩展，添加 Gemini CLI 常用的模型别名和能力配置。
///
/// 注意：Gemini CLI 使用 CloudCode PA 后端，但 OAuth client_id 权限受限，
/// 只能使用 Gemini 系列模型，不支持 Claude 模型。
pub struct GeminiCliModelResolver {
    inner: DefaultModelResolver,
}

impl GeminiCliModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // === 模型别名 ===
        // 简化的常用别名
        inner.extend_aliases(vec![
            ("gemini-pro", "gemini-2.5-pro"),
            ("gemini-flash", "gemini-2.5-flash"),
            ("gemini-2-pro", "gemini-2.0-pro-exp-02-05"),
            ("gemini-2-flash", "gemini-2.0-flash"),
            ("gemini-thinking", "gemini-2.0-flash-thinking-exp-01-21"),
        ]);

        // === 能力配置 ===
        // Gemini 2.5 系列
        inner.extend_capabilities(vec![
            ("gemini-2.5-pro", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING | Capability::THINKING),
            ("gemini-2.5-flash", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING),
        ]);

        // Gemini 2.0 系列
        inner.extend_capabilities(vec![
            ("gemini-2.0-flash", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING),
            ("gemini-2.0-pro-exp-02-05", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING | Capability::THINKING),
            ("gemini-2.0-flash-thinking-exp-01-21", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING | Capability::THINKING),
        ]);

        // Gemini 1.5 系列
        inner.extend_capabilities(vec![
            ("gemini-1.5-pro", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING),
            ("gemini-1.5-flash", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING),
            ("gemini-1.5-pro-002", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING),
            ("gemini-1.5-flash-002", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING),
        ]);

        // === 上下文长度 ===
        // Gemini 2.5 系列 (1M context)
        inner.extend_context_lengths(vec![
            ("gemini-2.5-pro", 1_000_000),
            ("gemini-2.5-flash", 1_000_000),
        ]);

        // Gemini 2.0 系列 (1M context)
        inner.extend_context_lengths(vec![
            ("gemini-2.0-flash", 1_000_000),
            ("gemini-2.0-pro-exp-02-05", 1_000_000),
            ("gemini-2.0-flash-thinking-exp-01-21", 1_000_000),
        ]);

        // Gemini 1.5 系列
        inner.extend_context_lengths(vec![
            ("gemini-1.5-pro", 1_000_000),
            ("gemini-1.5-flash", 1_000_000),
            ("gemini-1.5-pro-002", 1_000_000),
            ("gemini-1.5-flash-002", 1_000_000),
        ]);

        Self { inner }
    }
}

impl Default for GeminiCliModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for GeminiCliModelResolver {
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
