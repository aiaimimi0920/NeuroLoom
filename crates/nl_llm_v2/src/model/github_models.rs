use super::default::DefaultModelResolver;
use super::resolver::{Capability, Modality, ModelResolver};

/// GitHub Models 专属模型解析器
///
/// GitHub Models 使用 `provider/model` 风格的模型 ID，
/// 例如 `openai/gpt-4o-mini`、`meta/llama-3.3-70b-instruct`。
pub struct GitHubModelsModelResolver {
    inner: DefaultModelResolver,
}

impl GitHubModelsModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        inner.extend_aliases(vec![
            // OpenAI
            ("gpt4o-mini", "openai/gpt-4o-mini"),
            ("gpt-4o-mini", "openai/gpt-4o-mini"),
            ("gpt4.1-mini", "openai/gpt-4.1-mini"),
            ("gpt-4.1-mini", "openai/gpt-4.1-mini"),
            // Anthropic
            ("claude-sonnet", "anthropic/claude-3.7-sonnet"),
            ("claude-3.7-sonnet", "anthropic/claude-3.7-sonnet"),
            // Meta
            ("llama", "meta/llama-3.3-70b-instruct"),
            ("llama-3.3", "meta/llama-3.3-70b-instruct"),
            // Microsoft Phi
            ("phi-4", "microsoft/phi-4"),
            // DeepSeek
            ("deepseek-r1", "deepseek/deepseek-r1"),
        ]);

        let text_caps = Capability::CHAT | Capability::STREAMING;
        let tool_caps = text_caps | Capability::TOOLS;
        let vision_caps = tool_caps | Capability::VISION;
        let thinking_caps = text_caps | Capability::THINKING;

        inner.extend_capabilities(vec![
            ("openai/gpt-4o-mini", vision_caps),
            ("openai/gpt-4.1-mini", vision_caps),
            ("anthropic/claude-3.7-sonnet", vision_caps),
            ("meta/llama-3.3-70b-instruct", text_caps),
            ("microsoft/phi-4", tool_caps),
            ("deepseek/deepseek-r1", thinking_caps),
        ]);

        inner.extend_context_lengths(vec![
            ("openai/gpt-4o-mini", 128_000),
            ("openai/gpt-4.1-mini", 128_000),
            ("anthropic/claude-3.7-sonnet", 200_000),
            ("meta/llama-3.3-70b-instruct", 128_000),
            ("microsoft/phi-4", 128_000),
            ("deepseek/deepseek-r1", 64_000),
        ]);

        inner.extend_intelligence_profiles(vec![
            ("openai/gpt-4o-mini", 4.0, Modality::Multimodal),
            ("openai/gpt-4.1-mini", 4.1, Modality::Multimodal),
            ("anthropic/claude-3.7-sonnet", 4.7, Modality::Multimodal),
            ("meta/llama-3.3-70b-instruct", 3.8, Modality::Text),
            ("microsoft/phi-4", 3.9, Modality::Text),
            ("deepseek/deepseek-r1", 4.6, Modality::Text),
        ]);

        Self { inner }
    }
}

impl Default for GitHubModelsModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for GitHubModelsModelResolver {
    fn resolve(&self, model: &str) -> String {
        self.inner.resolve(&model.to_lowercase())
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

    fn intelligence_and_modality(&self, model: &str) -> Option<(f32, Modality)> {
        self.inner.intelligence_and_modality(model)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_models_aliases_resolve() {
        let resolver = GitHubModelsModelResolver::new();
        assert_eq!(resolver.resolve("gpt-4o-mini"), "openai/gpt-4o-mini");
        assert_eq!(resolver.resolve("LLAMA"), "meta/llama-3.3-70b-instruct");
    }

    #[test]
    fn github_models_capability_and_context() {
        let resolver = GitHubModelsModelResolver::new();
        assert!(resolver.has_capability("claude-sonnet", Capability::VISION));
        assert_eq!(resolver.max_context("deepseek-r1"), 64_000);
    }
}
