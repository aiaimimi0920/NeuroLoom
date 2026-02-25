use crate::model::{Capability, DefaultModelResolver, ModelResolver};

/// Nvidia NIM 模型解析器
///
/// ## 别名
///
/// | 别名 | 解析为 |
/// |------|--------|
/// | `nvidia` / `llama` | `meta/llama-3.3-70b-instruct` |
/// | `llama-405b` | `meta/llama-3.1-405b-instruct` |
/// | `llama-8b` | `meta/llama-3.1-8b-instruct` |
/// | `nemotron` | `nvidia/llama-3.1-nemotron-70b-instruct` |
/// | `deepseek` | `deepseek-ai/deepseek-r1` |
/// | `qwen` | `qwen/qwen2.5-72b-instruct` |
/// | `gemma` | `google/gemma-2-27b-it` |
/// | `mistral` | `mistralai/mistral-large-2-instruct` |
pub struct NvidiaModelResolver {
    inner: DefaultModelResolver,
}

impl NvidiaModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        inner.extend_aliases(vec![
            ("nvidia", "meta/llama-3.3-70b-instruct"),
            ("llama", "meta/llama-3.3-70b-instruct"),
            ("llama-70b", "meta/llama-3.3-70b-instruct"),
            ("llama-405b", "meta/llama-3.1-405b-instruct"),
            ("llama-8b", "meta/llama-3.1-8b-instruct"),
            ("nemotron", "nvidia/llama-3.1-nemotron-70b-instruct"),
            ("deepseek", "deepseek-ai/deepseek-r1"),
            ("r1", "deepseek-ai/deepseek-r1"),
            ("qwen", "qwen/qwen2.5-72b-instruct"),
            ("gemma", "google/gemma-2-27b-it"),
            ("mistral", "mistralai/mistral-large-2-instruct"),
        ]);

        let standard_caps = Capability::CHAT | Capability::TOOLS | Capability::STREAMING;
        let thinking_caps = standard_caps | Capability::THINKING;

        inner.extend_capabilities(vec![
            ("meta/llama-3.3-70b-instruct", standard_caps),
            ("meta/llama-3.1-405b-instruct", standard_caps),
            ("meta/llama-3.1-70b-instruct", standard_caps),
            ("meta/llama-3.1-8b-instruct", standard_caps),
            ("nvidia/llama-3.1-nemotron-70b-instruct", standard_caps),
            ("deepseek-ai/deepseek-r1", thinking_caps),
            ("qwen/qwen2.5-72b-instruct", standard_caps),
            ("google/gemma-2-27b-it", standard_caps),
            ("mistralai/mistral-large-2-instruct", standard_caps),
        ]);

        inner.extend_context_lengths(vec![
            ("meta/llama-3.3-70b-instruct", 128_000),
            ("meta/llama-3.1-405b-instruct", 128_000),
            ("meta/llama-3.1-70b-instruct", 128_000),
            ("meta/llama-3.1-8b-instruct", 128_000),
            ("nvidia/llama-3.1-nemotron-70b-instruct", 128_000),
            ("deepseek-ai/deepseek-r1", 64_000),
            ("qwen/qwen2.5-72b-instruct", 128_000),
            ("google/gemma-2-27b-it", 8_000),
            ("mistralai/mistral-large-2-instruct", 128_000),
        ]);

        Self { inner }
    }
}

impl Default for NvidiaModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for NvidiaModelResolver {
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
