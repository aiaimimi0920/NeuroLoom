//! OpenRouter 模型解析器
//!
//! OpenRouter 支持多个后端提供商的模型，格式为 "provider/model-name"

use super::default::DefaultModelResolver;
use super::resolver::{ModelResolver, Capability};

/// OpenRouter 模型解析器
///
/// OpenRouter 模型 ID 格式: `provider/model-name`
///
/// ## 模型变体
///
/// | 后缀 | 说明 |
/// |------|------|
/// | `:free` | 免费模型 |
/// | `:extended` | 扩展上下文窗口 |
/// | `:thinking` | 扩展推理能力 |
///
/// ## 别名
///
/// | 别名 | 解析为 |
/// |------|--------|
/// | `claude` / `claude-sonnet` | `anthropic/claude-3.5-sonnet` |
/// | `claude-opus` | `anthropic/claude-3-opus` |
/// | `claude-haiku` | `anthropic/claude-3.5-haiku` |
/// | `gpt4` / `gpt-4o` | `openai/gpt-4o` |
/// | `gemini` / `gemini-pro` | `google/gemini-2.5-pro` |
/// | `gemini-flash` | `google/gemini-2.5-flash` |
/// | `deepseek` | `deepseek/deepseek-chat` |
/// | `deepseek-r1` | `deepseek/deepseek-reasoner` |
///
/// ## 能力
///
/// - `CHAT`: 全部模型
/// - `VISION`: Claude、GPT-4o、Gemini 系列
/// - `TOOLS`: 大部分模型
/// - `STREAMING`: 全部模型
/// - `THINKING`: DeepSeek R1、Gemini 2.5 系列
pub struct OpenRouterModelResolver {
    inner: DefaultModelResolver,
}

impl OpenRouterModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // === 模型别名 ===
        inner.extend_aliases(vec![
            // Anthropic
            ("claude", "anthropic/claude-3.5-sonnet"),
            ("claude-sonnet", "anthropic/claude-3.5-sonnet"),
            ("claude-opus", "anthropic/claude-3-opus"),
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
            ("deepseek-r1", "deepseek/deepseek-reasoner"),
            ("r1", "deepseek/deepseek-reasoner"),
            // Mistral
            ("mistral", "mistralai/mistral-large"),
        ]);

        // === 能力配置 ===
        let claude_caps = Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING;
        let gpt_vision_caps = Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING;
        let gemini_caps = Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING | Capability::THINKING;
        let standard_caps = Capability::CHAT | Capability::TOOLS | Capability::STREAMING;
        let thinking_caps = Capability::CHAT | Capability::STREAMING | Capability::THINKING;

        inner.extend_capabilities(vec![
            // Anthropic Claude
            ("anthropic/claude-3-opus", claude_caps),
            ("anthropic/claude-3.5-sonnet", claude_caps),
            ("anthropic/claude-3.5-haiku", claude_caps),
            // OpenAI GPT
            ("openai/gpt-4o", gpt_vision_caps),
            ("openai/gpt-4-turbo", gpt_vision_caps),
            ("openai/gpt-3.5-turbo", standard_caps),
            // Google Gemini
            ("google/gemini-2.5-pro", gemini_caps),
            ("google/gemini-2.5-flash", gemini_caps),
            // Meta Llama
            ("meta-llama/llama-3.1-405b-instruct", standard_caps),
            ("meta-llama/llama-3.1-70b-instruct", standard_caps),
            // DeepSeek
            ("deepseek/deepseek-chat", standard_caps),
            ("deepseek/deepseek-reasoner", thinking_caps),
        ]);

        // === 上下文长度 ===
        inner.extend_context_lengths(vec![
            // Anthropic Claude
            ("anthropic/claude-3-opus", 200_000),
            ("anthropic/claude-3.5-sonnet", 200_000),
            ("anthropic/claude-3.5-haiku", 200_000),
            // OpenAI GPT
            ("openai/gpt-4o", 128_000),
            ("openai/gpt-4-turbo", 128_000),
            ("openai/gpt-3.5-turbo", 16_000),
            // Google Gemini
            ("google/gemini-2.5-pro", 1_000_000),
            ("google/gemini-2.5-flash", 1_000_000),
            // Meta Llama
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
