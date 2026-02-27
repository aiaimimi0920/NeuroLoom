use crate::model::{Capability, DefaultModelResolver, ModelResolver};

/// ocoolAI 主流模型目录。
///
/// 该目录同时服务于：
/// - `OcoolAiModelResolver` 的能力/上下文注册
/// - `provider::ocoolai` 的 `list_models` 返回
///
/// 通过单一数据源避免 resolver 与 provider 两份静态表漂移。
pub(crate) fn mainstream_models() -> &'static [(&'static str, &'static str)] {
    &[
        ("gpt-4o", "GPT-4o — Flagship multimodal model, 128K context"),
        (
            "gpt-4o-mini",
            "GPT-4o Mini — Fast and affordable, 128K context",
        ),
        (
            "gpt-4-turbo",
            "GPT-4 Turbo — Previous generation, 128K context",
        ),
        (
            "gpt-3.5-turbo",
            "GPT-3.5 Turbo — Fast and economical, 16K context",
        ),
        (
            "claude-3-5-sonnet-20241022",
            "Claude 3.5 Sonnet — Latest Claude model, 200K context",
        ),
        (
            "claude-3-opus-20240229",
            "Claude 3 Opus — Most capable Claude model, 200K context",
        ),
        (
            "claude-3-haiku-20240307",
            "Claude 3 Haiku — Fast and efficient, 200K context",
        ),
        (
            "gemini-1.5-pro",
            "Gemini 1.5 Pro — Advanced reasoning, 1M context",
        ),
        (
            "gemini-1.5-flash",
            "Gemini 1.5 Flash — Fast and efficient, 1M context",
        ),
        (
            "deepseek-chat",
            "DeepSeek V3 — General purpose chat, 64K context",
        ),
        (
            "deepseek-reasoner",
            "DeepSeek R1 — Deep reasoning model, 64K context",
        ),
        (
            "llama-3.1-405b",
            "Llama 3.1 405B — Largest Llama model, 128K context",
        ),
        ("qwen-max", "Qwen Max — Advanced reasoning, 32K context"),
        ("glm-4", "GLM-4 — Zhipu AI flagship model, 128K context"),
    ]
}

/// ocoolAI 模型解析器
///
/// ocoolAI 平台支持 200+ 模型，这里定义主流模型的别名、能力和上下文长度。
pub struct OcoolAiModelResolver {
    inner: DefaultModelResolver,
}

impl OcoolAiModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // === 模型别名 ===
        inner.extend_aliases(vec![
            // 平台别名
            ("ocool", "gpt-4o-mini"),
            ("ocoolai", "gpt-4o-mini"),
            // GPT-4 系列
            ("gpt4", "gpt-4o"),
            ("gpt-4", "gpt-4o"),
            ("gpt4o", "gpt-4o"),
            ("4o", "gpt-4o"),
            ("4o-mini", "gpt-4o-mini"),
            ("gpt-4o-mini", "gpt-4o-mini"),
            ("gpt-4-turbo", "gpt-4-turbo"),
            ("gpt4-turbo", "gpt-4-turbo"),
            // GPT-3.5 系列
            ("gpt3", "gpt-3.5-turbo"),
            ("gpt-3.5", "gpt-3.5-turbo"),
            ("gpt35", "gpt-3.5-turbo"),
            ("3.5", "gpt-3.5-turbo"),
            // Claude 系列
            ("claude", "claude-3-5-sonnet-20241022"),
            ("sonnet", "claude-3-5-sonnet-20241022"),
            ("claude-3-sonnet", "claude-3-5-sonnet-20241022"),
            ("claude-3.5-sonnet", "claude-3-5-sonnet-20241022"),
            ("opus", "claude-3-opus-20240229"),
            ("claude-3-opus", "claude-3-opus-20240229"),
            ("haiku", "claude-3-haiku-20240307"),
            ("claude-3-haiku", "claude-3-haiku-20240307"),
            // Gemini 系列
            ("gemini", "gemini-1.5-flash"),
            ("gemini-pro", "gemini-1.5-pro"),
            ("gemini-flash", "gemini-1.5-flash"),
            ("gemini-1.5-pro", "gemini-1.5-pro"),
            ("gemini-1.5-flash", "gemini-1.5-flash"),
            // DeepSeek 系列
            ("deepseek", "deepseek-chat"),
            ("ds", "deepseek-chat"),
            ("deepseek-chat", "deepseek-chat"),
            ("deepseek-v3", "deepseek-chat"),
            ("r1", "deepseek-reasoner"),
            ("think", "deepseek-reasoner"),
            ("deepseek-r1", "deepseek-reasoner"),
            ("deepseek-reasoner", "deepseek-reasoner"),
            // Llama 系列
            ("llama", "llama-3.1-405b"),
            ("llama-3.1", "llama-3.1-405b"),
            ("llama-3.1-405b", "llama-3.1-405b"),
            // Qwen 系列
            ("qwen", "qwen-max"),
            ("qwen-max", "qwen-max"),
            ("qwen-plus", "qwen-plus"),
            // GLM 系列
            ("glm", "glm-4"),
            ("glm-4", "glm-4"),
            ("glm-4-plus", "glm-4-plus"),
        ]);

        // 标准能力集
        let tool_caps = Capability::CHAT | Capability::STREAMING | Capability::TOOLS;
        let vision_caps =
            Capability::CHAT | Capability::STREAMING | Capability::VISION | Capability::TOOLS;
        let thinking_caps = Capability::CHAT | Capability::STREAMING | Capability::THINKING;

        // === 能力配置 ===
        inner.extend_capabilities(vec![
            // GPT-4o 系列
            ("gpt-4o", vision_caps),
            ("gpt-4o-mini", vision_caps),
            ("gpt-4-turbo", vision_caps),
            ("gpt-4-turbo-preview", tool_caps),
            ("gpt-4-0125-preview", tool_caps),
            // GPT-3.5 系列
            ("gpt-3.5-turbo", tool_caps),
            ("gpt-3.5-turbo-0125", tool_caps),
            // Claude 3 系列
            ("claude-3-5-sonnet-20241022", vision_caps),
            ("claude-3-opus-20240229", vision_caps),
            ("claude-3-haiku-20240307", vision_caps),
            ("claude-3-sonnet-20240229", vision_caps),
            // Claude 3.5 系列
            ("claude-3-5-sonnet", vision_caps),
            ("claude-3-5-haiku", vision_caps),
            // Gemini 系列
            ("gemini-1.5-pro", vision_caps),
            ("gemini-1.5-flash", vision_caps),
            ("gemini-pro", vision_caps),
            // DeepSeek 系列
            ("deepseek-chat", tool_caps),
            ("deepseek-reasoner", thinking_caps),
            // Llama 系列
            ("llama-3.1-405b", tool_caps),
            ("llama-3.1-70b", tool_caps),
            ("llama-3.1-8b", tool_caps),
            // Qwen 系列
            ("qwen-max", tool_caps),
            ("qwen-plus", tool_caps),
            ("qwen-turbo", tool_caps),
            // GLM 系列
            ("glm-4", tool_caps),
            ("glm-4-plus", tool_caps),
            ("glm-4-air", tool_caps),
        ]);

        // === 上下文长度 ===
        inner.extend_context_lengths(vec![
            // GPT-4o 系列
            ("gpt-4o", 128_000),
            ("gpt-4o-mini", 128_000),
            ("gpt-4-turbo", 128_000),
            ("gpt-4-turbo-preview", 128_000),
            ("gpt-4-0125-preview", 128_000),
            // GPT-3.5 系列
            ("gpt-3.5-turbo", 16_385),
            ("gpt-3.5-turbo-0125", 16_385),
            // Claude 系列
            ("claude-3-5-sonnet-20241022", 200_000),
            ("claude-3-opus-20240229", 200_000),
            ("claude-3-haiku-20240307", 200_000),
            ("claude-3-sonnet-20240229", 200_000),
            ("claude-3-5-sonnet", 200_000),
            ("claude-3-5-haiku", 200_000),
            // Gemini 系列
            ("gemini-1.5-pro", 1_000_000),
            ("gemini-1.5-flash", 1_000_000),
            ("gemini-pro", 32_000),
            // DeepSeek 系列
            ("deepseek-chat", 64_000),
            ("deepseek-reasoner", 64_000),
            // Llama 系列
            ("llama-3.1-405b", 128_000),
            ("llama-3.1-70b", 128_000),
            ("llama-3.1-8b", 128_000),
            // Qwen 系列
            ("qwen-max", 32_000),
            ("qwen-plus", 32_000),
            ("qwen-turbo", 8_000),
            // GLM 系列
            ("glm-4", 128_000),
            ("glm-4-plus", 128_000),
            ("glm-4-air", 128_000),
        ]);

        Self { inner }
    }
}

impl Default for OcoolAiModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for OcoolAiModelResolver {
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
        model: &str,
    ) -> Option<(f32, crate::model::resolver::Modality)> {
        self.inner.intelligence_and_modality(model)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_common_aliases() {
        let resolver = OcoolAiModelResolver::new();
        // 平台别名
        assert_eq!(resolver.resolve("ocool"), "gpt-4o-mini");
        assert_eq!(resolver.resolve("ocoolai"), "gpt-4o-mini");
        // GPT
        assert_eq!(resolver.resolve("4o"), "gpt-4o");
        assert_eq!(resolver.resolve("3.5"), "gpt-3.5-turbo");
        // Claude
        assert_eq!(resolver.resolve("claude"), "claude-3-5-sonnet-20241022");
        assert_eq!(resolver.resolve("sonnet"), "claude-3-5-sonnet-20241022");
        // DeepSeek
        assert_eq!(resolver.resolve("ds"), "deepseek-chat");
        assert_eq!(resolver.resolve("r1"), "deepseek-reasoner");
        assert_eq!(resolver.resolve("think"), "deepseek-reasoner");
    }

    #[test]
    fn provides_capabilities() {
        let resolver = OcoolAiModelResolver::new();
        // GPT-4o 支持视觉和工具
        assert!(resolver.has_capability("gpt-4o", Capability::VISION));
        assert!(resolver.has_capability("gpt-4o", Capability::TOOLS));
        assert!(resolver.has_capability("4o", Capability::VISION));
        // DeepSeek R1 支持思考
        assert!(resolver.has_capability("deepseek-reasoner", Capability::THINKING));
        assert!(resolver.has_capability("r1", Capability::THINKING));
    }

    #[test]
    fn provides_context_lengths() {
        let resolver = OcoolAiModelResolver::new();
        assert_eq!(resolver.max_context("gpt-4o"), 128_000);
        assert_eq!(resolver.max_context("claude"), 200_000);
        assert_eq!(resolver.max_context("gemini"), 1_000_000);
        assert_eq!(resolver.max_context("ds"), 64_000);
    }

    #[test]
    fn mainstream_catalog_contains_default_model() {
        let has_default = mainstream_models()
            .iter()
            .any(|(id, _)| *id == "gpt-4o-mini");
        assert!(has_default);
    }
}
