//! OpenRouter 模型解析器
//!
//! OpenRouter 支持多个后端提供商的模型，格式为 "provider/model-name"

use super::default::DefaultModelResolver;
use super::resolver::{ModelResolver, Capability};

/// OpenRouter 模型解析器
///
/// OpenRouter 模型 ID 格式: "provider/model-name"
/// 例如: "anthropic/claude-3-opus", "openai/gpt-4o", "google/gemini-2.5-pro"
pub struct OpenRouterModelResolver {
    inner: DefaultModelResolver,
}

impl OpenRouterModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // === 常用模型别名 ===
        // Anthropic
        inner.extend_aliases(vec![
            ("claude", "anthropic/claude-3-opus"),
            ("claude-opus", "anthropic/claude-3-opus"),
            ("claude-sonnet", "anthropic/claude-3.5-sonnet"),
            ("claude-haiku", "anthropic/claude-3.5-haiku"),
            // OpenAI
            ("gpt4", "openai/gpt-4o"),
            ("gpt-4o", "openai/gpt-4o"),
            ("gpt4o", "openai/gpt-4o"),
            ("gpt-4-turbo", "openai/gpt-4-turbo"),
            ("gpt-3.5", "openai/gpt-3.5-turbo"),
            // Google
            ("gemini", "google/gemini-2.5-pro"),
            ("gemini-pro", "google/gemini-2.5-pro"),
            ("gemini-flash", "google/gemini-2.5-flash"),
            // Meta
            ("llama", "meta-llama/llama-3.1-405b-instruct"),
            ("llama-70b", "meta-llama/llama-3.1-70b-instruct"),
            // DeepSeek
            ("deepseek", "deepseek/deepseek-chat"),
            // Mistral
            ("mistral", "mistralai/mistral-large"),
        ]);

        // === 能力配置 ===
        // Anthropic
        inner.extend_capabilities(vec![
            ("anthropic/claude-3-opus", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING),
            ("anthropic/claude-3.5-sonnet", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING),
            ("anthropic/claude-3.5-haiku", Capability::CHAT | Capability::TOOLS | Capability::STREAMING),
            // OpenAI
            ("openai/gpt-4o", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING),
            ("openai/gpt-4-turbo", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING),
            ("openai/gpt-3.5-turbo", Capability::CHAT | Capability::TOOLS | Capability::STREAMING),
            // Google
            ("google/gemini-2.5-pro", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING | Capability::THINKING),
            ("google/gemini-2.5-flash", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING | Capability::THINKING),
            // Meta
            ("meta-llama/llama-3.1-405b-instruct", Capability::CHAT | Capability::TOOLS | Capability::STREAMING),
            ("meta-llama/llama-3.1-70b-instruct", Capability::CHAT | Capability::TOOLS | Capability::STREAMING),
            // DeepSeek
            ("deepseek/deepseek-chat", Capability::CHAT | Capability::TOOLS | Capability::STREAMING),
            ("deepseek/deepseek-reasoner", Capability::CHAT | Capability::STREAMING | Capability::THINKING),
        ]);

        // === 上下文长度 ===
        inner.extend_context_lengths(vec![
            // Anthropic
            ("anthropic/claude-3-opus", 200_000),
            ("anthropic/claude-3.5-sonnet", 200_000),
            ("anthropic/claude-3.5-haiku", 200_000),
            // OpenAI
            ("openai/gpt-4o", 128_000),
            ("openai/gpt-4-turbo", 128_000),
            ("openai/gpt-3.5-turbo", 16_000),
            // Google
            ("google/gemini-2.5-pro", 1_000_000),
            ("google/gemini-2.5-flash", 1_000_000),
            // Meta
            ("meta-llama/llama-3.1-405b-instruct", 128_000),
            ("meta-llama/llama-3.1-70b-instruct", 128_000),
            // DeepSeek
            ("deepseek/deepseek-chat", 64_000),
            ("deepseek/deepseek-reasoner", 64_000),
        ]);

        Self { inner }
    }
}

impl Default for OpenRouterModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for OpenRouterModelResolver {
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
