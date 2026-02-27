use super::default::DefaultModelResolver;
use super::resolver::{Capability, ModelResolver};

/// Cephalon 模型解析器
///
/// Cephalon 是一个 AI 模型聚合平台，支持多种主流 LLM 模型。
///
/// 支持的模型类别：
/// - OpenAI 系列：gpt-4o, gpt-4o-mini, gpt-4-turbo, gpt-3.5-turbo
/// - Claude 系列：claude-3-opus, claude-3-sonnet, claude-3-haiku
/// - DeepSeek 系列：deepseek-chat, deepseek-reasoner
/// - Gemini 系列：gemini-1.5-pro, gemini-1.5-flash
pub struct CephalonModelResolver {
    inner: DefaultModelResolver,
}

impl CephalonModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // === 模型别名 ===
        inner.extend_aliases(vec![
            // OpenAI 系列别名
            ("gpt4", "gpt-4o"),
            ("gpt-4", "gpt-4o"),
            ("gpt4o", "gpt-4o"),
            ("gpt4-mini", "gpt-4o-mini"),
            ("gpt-4-turbo", "gpt-4-turbo"),
            ("gpt3", "gpt-3.5-turbo"),
            ("gpt-3.5", "gpt-3.5-turbo"),
            ("gpt35", "gpt-3.5-turbo"),
            // Claude 系列别名
            ("claude3-opus", "claude-3-opus-20240229"),
            ("claude-3-opus", "claude-3-opus-20240229"),
            ("claude3-sonnet", "claude-3-sonnet-20240229"),
            ("claude-3-sonnet", "claude-3-sonnet-20240229"),
            ("claude3-haiku", "claude-3-haiku-20240307"),
            ("claude-3-haiku", "claude-3-haiku-20240307"),
            // DeepSeek 系列别名
            ("deepseek", "deepseek-chat"),
            ("ds", "deepseek-chat"),
            ("reasoner", "deepseek-reasoner"),
            ("r1", "deepseek-reasoner"),
            // Gemini 系列别名
            ("gemini", "gemini-1.5-pro"),
            ("gemini-pro", "gemini-1.5-pro"),
            ("gemini-flash", "gemini-1.5-flash"),
        ]);

        // === 能力配置 ===
        // OpenAI 系列
        inner.extend_capabilities(vec![
            (
                "gpt-4o",
                Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING,
            ),
            (
                "gpt-4o-mini",
                Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING,
            ),
            (
                "gpt-4-turbo",
                Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING,
            ),
            (
                "gpt-3.5-turbo",
                Capability::CHAT | Capability::TOOLS | Capability::STREAMING,
            ),
            // Claude 系列
            (
                "claude-3-opus-20240229",
                Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING,
            ),
            (
                "claude-3-sonnet-20240229",
                Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING,
            ),
            (
                "claude-3-haiku-20240307",
                Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING,
            ),
            // DeepSeek 系列
            (
                "deepseek-chat",
                Capability::CHAT | Capability::TOOLS | Capability::STREAMING,
            ),
            (
                "deepseek-reasoner",
                Capability::CHAT | Capability::STREAMING | Capability::THINKING,
            ),
            // Gemini 系列
            (
                "gemini-1.5-pro",
                Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING,
            ),
            (
                "gemini-1.5-flash",
                Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING,
            ),
        ]);

        // === 上下文长度 ===
        inner.extend_context_lengths(vec![
            // OpenAI 系列
            ("gpt-4o", 128_000),
            ("gpt-4o-mini", 128_000),
            ("gpt-4-turbo", 128_000),
            ("gpt-3.5-turbo", 16_385),
            // Claude 系列
            ("claude-3-opus-20240229", 200_000),
            ("claude-3-sonnet-20240229", 200_000),
            ("claude-3-haiku-20240307", 200_000),
            // DeepSeek 系列
            ("deepseek-chat", 64_000),
            ("deepseek-reasoner", 64_000),
            // Gemini 系列
            ("gemini-1.5-pro", 1_000_000),
            ("gemini-1.5-flash", 1_000_000),
        ]);

        Self { inner }
    }
}

impl Default for CephalonModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for CephalonModelResolver {
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
