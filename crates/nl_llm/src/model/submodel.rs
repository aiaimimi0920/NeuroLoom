use super::default::DefaultModelResolver;
use super::resolver::{Capability, Modality, ModelResolver};

/// SubModel 模型解析器
///
/// SubModel 代理了一系列开源和闭源的高能模型。
pub struct SubModelModelResolver {
    inner: DefaultModelResolver,
}

impl SubModelModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // ========== 别名映射 ==========
        inner.extend_aliases(vec![
            ("hermes", "NousResearch/Hermes-4-405B-FP8"),
            ("hermes-4-405b-fp8", "NousResearch/Hermes-4-405B-FP8"),
            ("qwen-thinking", "Qwen/Qwen3-235B-A22B-Thinking-2507"),
            ("qwen-3-thinking", "Qwen/Qwen3-235B-A22B-Thinking-2507"),
            ("qwen-coder", "Qwen/Qwen3-Coder-480B-A35B-Instruct-FP8"),
            ("qwen3-coder", "Qwen/Qwen3-Coder-480B-A35B-Instruct-FP8"),
            ("qwen", "Qwen/Qwen3-235B-A22B-Instruct-2507"),
            ("qwen3", "Qwen/Qwen3-235B-A22B-Instruct-2507"),
            ("glm", "zai-org/GLM-4.5-FP8"),
            ("glm-4.5", "zai-org/GLM-4.5-FP8"),
            ("gpt-oss", "openai/gpt-oss-120b"),
            ("gpt-oss-120b", "openai/gpt-oss-120b"),
            ("r1", "deepseek-ai/DeepSeek-R1"),
            ("deepseek-r1", "deepseek-ai/DeepSeek-R1"),
            ("ds", "deepseek-ai/DeepSeek-V3.1"),
            ("v3", "deepseek-ai/DeepSeek-V3.1"),
            ("deepseek-v3", "deepseek-ai/DeepSeek-V3.1"),
        ]);

        // ========== 能力配置 ==========
        let text_caps = Capability::CHAT | Capability::STREAMING | Capability::TOOLS;
        inner.extend_capabilities(vec![
            ("NousResearch/Hermes-4-405B-FP8", text_caps),
            (
                "Qwen/Qwen3-235B-A22B-Thinking-2507",
                text_caps | Capability::THINKING,
            ),
            ("Qwen/Qwen3-Coder-480B-A35B-Instruct-FP8", text_caps),
            ("Qwen/Qwen3-235B-A22B-Instruct-2507", text_caps),
            ("zai-org/GLM-4.5-FP8", text_caps),
            ("openai/gpt-oss-120b", text_caps),
            ("deepseek-ai/DeepSeek-R1", text_caps | Capability::THINKING),
            ("deepseek-ai/DeepSeek-V3.1", text_caps),
        ]);

        // ========== 上下文长度 ==========
        inner.extend_context_lengths(vec![
            ("NousResearch/Hermes-4-405B-FP8", 128_000),
            ("Qwen/Qwen3-235B-A22B-Thinking-2507", 128_000),
            ("Qwen/Qwen3-Coder-480B-A35B-Instruct-FP8", 128_000),
            ("Qwen/Qwen3-235B-A22B-Instruct-2507", 128_000),
            ("zai-org/GLM-4.5-FP8", 64_000),
            ("openai/gpt-oss-120b", 128_000),
            ("deepseek-ai/DeepSeek-R1", 64_000),
            ("deepseek-ai/DeepSeek-V3.1", 64_000),
        ]);

        // ========== 智能等级与模态 ==========
        inner.extend_intelligence_profiles(vec![
            ("NousResearch/Hermes-4-405B-FP8", 4.0, Modality::Text),
            ("Qwen/Qwen3-235B-A22B-Thinking-2507", 4.5, Modality::Text),
            (
                "Qwen/Qwen3-Coder-480B-A35B-Instruct-FP8",
                4.4,
                Modality::Text,
            ),
            ("Qwen/Qwen3-235B-A22B-Instruct-2507", 4.3, Modality::Text),
            ("zai-org/GLM-4.5-FP8", 4.5, Modality::Text),
            ("openai/gpt-oss-120b", 4.2, Modality::Text),
            ("deepseek-ai/DeepSeek-R1", 4.5, Modality::Text),
            ("deepseek-ai/DeepSeek-V3.1", 4.5, Modality::Text),
        ]);

        Self { inner }
    }
}

impl Default for SubModelModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for SubModelModelResolver {
    fn resolve(&self, model: &str) -> String {
        self.inner.resolve(&model.to_lowercase())
    }

    fn has_capability(&self, model: &str, capability: Capability) -> bool {
        self.inner.has_capability(model, capability)
    }

    fn context_window_hint(&self, model: &str) -> (usize, usize) {
        self.inner.context_window_hint(model)
    }

    fn max_context(&self, model: &str) -> usize {
        self.inner.max_context(model)
    }

    fn intelligence_and_modality(&self, model: &str) -> Option<(f32, Modality)> {
        self.inner.intelligence_and_modality(model)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn submodel_aliases_resolve() {
        let resolver = SubModelModelResolver::new();
        assert_eq!(
            resolver.resolve("qwen-thinking"),
            "Qwen/Qwen3-235B-A22B-Thinking-2507"
        );
        assert_eq!(resolver.resolve("DS"), "deepseek-ai/DeepSeek-V3.1");
    }

    #[test]
    fn submodel_capability_and_context() {
        let resolver = SubModelModelResolver::new();
        assert!(resolver.has_capability("r1", Capability::THINKING));
        assert_eq!(resolver.max_context("deepseek-r1"), 64_000);
    }
}
