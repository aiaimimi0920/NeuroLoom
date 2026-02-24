use super::default::DefaultModelResolver;
use super::resolver::{ModelResolver, Capability};

/// Qwen Code 平台专属模型解析器
///
/// portal.qwen.ai OAuth 端点使用的模型名与 DashScope API 不同：
/// - qwen3-coder-plus: 高级代码生成模型
/// - qwen3-coder-flash: 快速代码生成模型
/// - coder-model: Qwen 3.5 Plus（最新最强）
/// - vision-model: 视觉模型
pub struct QwenModelResolver {
    inner: DefaultModelResolver,
}

impl QwenModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // === 模型别名 ===
        inner.extend_aliases(vec![
            ("qwen", "qwen3-coder-plus"),
            ("qwen-coder", "qwen3-coder-plus"),
            ("qwen-coder-plus", "qwen3-coder-plus"),
            ("qwen-coder-flash", "qwen3-coder-flash"),
            ("qwen-coder-model", "coder-model"),
            ("qwen-vision", "vision-model"),
        ]);

        // === 能力配置 ===
        inner.extend_capabilities(vec![
            ("qwen3-coder-plus", Capability::CHAT | Capability::TOOLS | Capability::STREAMING | Capability::THINKING),
            ("qwen3-coder-flash", Capability::CHAT | Capability::TOOLS | Capability::STREAMING),
            ("coder-model", Capability::CHAT | Capability::TOOLS | Capability::STREAMING | Capability::THINKING),
            ("vision-model", Capability::CHAT | Capability::VISION | Capability::STREAMING),
        ]);

        // === 上下文长度 ===
        inner.extend_context_lengths(vec![
            ("qwen3-coder-plus", 32_768),
            ("qwen3-coder-flash", 8_192),
            ("coder-model", 1_048_576),
            ("vision-model", 32_768),
        ]);

        Self { inner }
    }
}

impl Default for QwenModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for QwenModelResolver {
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
