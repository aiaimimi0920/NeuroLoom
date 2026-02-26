use super::default::DefaultModelResolver;
use super::resolver::{ModelResolver, Capability};

/// iFlow 平台专属模型解析器
///
/// 基于 DefaultModelResolver 扩展，添加 iFlow 平台特有的模型别名和能力配置：
/// - Qwen 系列模型
/// - GLM 系列模型
/// - DeepSeek 系列模型
/// - MiniMax 系列模型
pub struct IFlowModelResolver {
    inner: DefaultModelResolver,
}

impl IFlowModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // === 模型别名 ===
        // Qwen 系列
        inner.extend_aliases(vec![
            ("qwen", "qwen3-max"),
            ("qwen-max", "qwen3-max"),
            ("qwen-turbo", "qwen3-turbo"),
            ("qwen-plus", "qwen3-plus"),
        ]);

        // GLM 系列
        inner.extend_aliases(vec![
            ("glm", "glm-4-flash"),
            ("glm-4-air", "glm-4-air"),
            ("glm-4-flash", "glm-4-flash"),
        ]);

        // DeepSeek 系列
        inner.extend_aliases(vec![
            ("deepseek", "deepseek-v3"),
            ("deepseek-chat", "deepseek-v3"),
            ("deepseek-reasoner", "deepseek-r1"),
        ]);

        // === 能力配置 ===
        // Qwen 系列 (支持 thinking)
        inner.extend_capabilities(vec![
            ("qwen3-max", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING | Capability::THINKING),
            ("qwen3-max-preview", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING | Capability::THINKING),
            ("qwen3-turbo", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING),
            ("qwen3-plus", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING),
            ("qwen2.5-max", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING),
            ("qwen2.5-turbo", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING),
        ]);

        // GLM 系列 (支持 thinking)
        inner.extend_capabilities(vec![
            ("glm-4", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING | Capability::THINKING),
            ("glm-4-air", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING | Capability::THINKING),
            ("glm-4-flash", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING | Capability::THINKING),
            ("glm-4-plus", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING | Capability::THINKING),
            ("glm-z1-air", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING | Capability::THINKING),
            ("glm-z1-flash", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING | Capability::THINKING),
        ]);

        // DeepSeek 系列 (支持 thinking)
        inner.extend_capabilities(vec![
            ("deepseek-v3", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING),
            ("deepseek-v3.1", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING),
            ("deepseek-v3.2", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING),
            ("deepseek-r1", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING | Capability::THINKING),
        ]);

        // MiniMax 系列
        inner.extend_capabilities(vec![
            ("minimax-text-01", Capability::CHAT | Capability::TOOLS | Capability::STREAMING),
            ("abab6.5s-chat", Capability::CHAT | Capability::TOOLS | Capability::STREAMING),
        ]);

        // === 上下文长度 ===
        // Qwen 系列
        inner.extend_context_lengths(vec![
            ("qwen3-max", 128_000),
            ("qwen3-max-preview", 128_000),
            ("qwen3-turbo", 128_000),
            ("qwen3-plus", 128_000),
            ("qwen2.5-max", 128_000),
            ("qwen2.5-turbo", 128_000),
        ]);

        // GLM 系列
        inner.extend_context_lengths(vec![
            ("glm-4", 128_000),
            ("glm-4-air", 128_000),
            ("glm-4-flash", 128_000),
            ("glm-4-plus", 128_000),
            ("glm-z1-air", 128_000),
            ("glm-z1-flash", 128_000),
        ]);

        // DeepSeek 系列
        inner.extend_context_lengths(vec![
            ("deepseek-v3", 128_000),
            ("deepseek-v3.1", 128_000),
            ("deepseek-v3.2", 128_000),
            ("deepseek-r1", 64_000),
        ]);

        // MiniMax 系列
        inner.extend_context_lengths(vec![
            ("minimax-text-01", 245_000),
            ("abab6.5s-chat", 245_000),
        ]);

        Self { inner }
    }
}

impl Default for IFlowModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for IFlowModelResolver {
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
