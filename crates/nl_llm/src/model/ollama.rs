use crate::model::{Capability, DefaultModelResolver, ModelResolver};

const DEFAULT_OLLAMA_MODEL: &str = "llama3";
const DEFAULT_CONTEXT: usize = 8192;

pub struct OllamaModelResolver {
    inner: DefaultModelResolver,
}

impl OllamaModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();
        let capability = Capability::CHAT | Capability::TOOLS | Capability::STREAMING;

        inner.extend_capabilities(vec![(DEFAULT_OLLAMA_MODEL, capability)]);
        inner.extend_context_lengths(vec![(DEFAULT_OLLAMA_MODEL, DEFAULT_CONTEXT)]);

        Self { inner }
    }
}

impl Default for OllamaModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for OllamaModelResolver {
    fn resolve(&self, model: &str) -> String {
        self.inner.resolve(model)
    }

    fn has_capability(&self, model: &str, capability: Capability) -> bool {
        self.inner.has_capability(model, capability)
            || capability == Capability::CHAT
            || capability == Capability::STREAMING
    }

    fn max_context(&self, model: &str) -> usize {
        let context = self.inner.max_context(model);
        context.max(DEFAULT_CONTEXT)
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
