use super::default::DefaultModelResolver;
use super::resolver::{Capability, ModelResolver};

/// OpenAI 模型解析器
///
/// 支持的模型（截至 2026-02-25）：
/// - GPT-4o 系列：gpt-4o, gpt-4o-mini
/// - GPT-4 Turbo 系列：gpt-4-turbo, gpt-4-0125-preview
/// - GPT-3.5 系列：gpt-3.5-turbo
/// - 推理模型：o1, o1-mini, o1-pro, o3-mini
pub struct OpenAiModelResolver {
    inner: DefaultModelResolver,
}

impl OpenAiModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // === 模型别名 ===
        inner.extend_aliases(vec![
            // GPT-4 系列
            ("gpt4", "gpt-4o"),
            ("gpt-4", "gpt-4o"),
            ("gpt4o", "gpt-4o"),
            ("gpt-4o-mini", "gpt-4o-mini"),
            ("gpt4-mini", "gpt-4o-mini"),
            ("gpt-4-turbo", "gpt-4-turbo"),
            ("gpt4-turbo", "gpt-4-turbo"),
            // GPT-3.5 系列
            ("gpt3", "gpt-3.5-turbo"),
            ("gpt-3.5", "gpt-3.5-turbo"),
            ("gpt35", "gpt-3.5-turbo"),
            // 推理模型
            ("o1", "o1"),
            ("o1-mini", "o1-mini"),
            ("o1-pro", "o1-pro"),
            ("o3", "o3-mini"),
            ("o3-mini", "o3-mini"),
        ]);

        // === 能力配置 ===
        // GPT-4o 系列
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
                "gpt-4-0125-preview",
                Capability::CHAT | Capability::TOOLS | Capability::STREAMING,
            ),
            // GPT-3.5 系列
            (
                "gpt-3.5-turbo",
                Capability::CHAT | Capability::TOOLS | Capability::STREAMING,
            ),
            (
                "gpt-3.5-turbo-0125",
                Capability::CHAT | Capability::TOOLS | Capability::STREAMING,
            ),
            // 推理模型
            (
                "o1",
                Capability::CHAT
                    | Capability::VISION
                    | Capability::TOOLS
                    | Capability::STREAMING
                    | Capability::THINKING,
            ),
            (
                "o1-mini",
                Capability::CHAT | Capability::STREAMING | Capability::THINKING,
            ),
            (
                "o1-pro",
                Capability::CHAT
                    | Capability::VISION
                    | Capability::TOOLS
                    | Capability::STREAMING
                    | Capability::THINKING,
            ),
            (
                "o3-mini",
                Capability::CHAT | Capability::TOOLS | Capability::STREAMING | Capability::THINKING,
            ),
        ]);

        // === 上下文长度 ===
        inner.extend_context_lengths(vec![
            // GPT-4o 系列
            ("gpt-4o", 128_000),
            ("gpt-4o-mini", 128_000),
            ("gpt-4-turbo", 128_000),
            ("gpt-4-0125-preview", 128_000),
            // GPT-3.5 系列
            ("gpt-3.5-turbo", 16_385),
            ("gpt-3.5-turbo-0125", 16_385),
            // 推理模型
            ("o1", 200_000),
            ("o1-mini", 128_000),
            ("o1-pro", 200_000),
            ("o3-mini", 200_000),
        ]);

        Self { inner }
    }
}

impl Default for OpenAiModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for OpenAiModelResolver {
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
