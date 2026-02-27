//! Groq 模型解析器
//!
//! Groq 提供 OpenAI 兼容接口，主打低延迟推理。

use crate::model::{Capability, DefaultModelResolver, ModelResolver};

#[derive(Debug, Clone, Copy)]
pub struct GroqModelMeta {
    pub id: &'static str,
    pub context: usize,
    pub supports_tools: bool,
}

pub const GROQ_MODEL_META: &[GroqModelMeta] = &[
    GroqModelMeta {
        id: "llama-3.3-70b-versatile",
        context: 128_000,
        supports_tools: true,
    },
    GroqModelMeta {
        id: "llama-3.1-8b-instant",
        context: 128_000,
        supports_tools: true,
    },
    GroqModelMeta {
        id: "mixtral-8x7b-32768",
        context: 32_768,
        supports_tools: false,
    },
    GroqModelMeta {
        id: "gemma2-9b-it",
        context: 8_192,
        supports_tools: false,
    },
];

fn capability_for(meta: &GroqModelMeta) -> Capability {
    let mut caps = Capability::CHAT | Capability::STREAMING;
    if meta.supports_tools {
        caps |= Capability::TOOLS;
    }
    caps
}

pub struct GroqModelResolver {
    inner: DefaultModelResolver,
}

impl GroqModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        inner.extend_aliases(vec![
            ("groq", "llama-3.3-70b-versatile"),
            ("llama-70b", "llama-3.3-70b-versatile"),
            ("llama-8b", "llama-3.1-8b-instant"),
            ("mixtral", "mixtral-8x7b-32768"),
            ("gemma", "gemma2-9b-it"),
        ]);

        inner.extend_capabilities(
            GROQ_MODEL_META
                .iter()
                .map(|meta| (meta.id, capability_for(meta)))
                .collect(),
        );

        inner.extend_context_lengths(
            GROQ_MODEL_META
                .iter()
                .map(|meta| (meta.id, meta.context))
                .collect(),
        );

        Self { inner }
    }
}

impl Default for GroqModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for GroqModelResolver {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliases_resolve_to_expected_models() {
        let resolver = GroqModelResolver::new();
        assert_eq!(resolver.resolve("groq"), "llama-3.3-70b-versatile");
        assert_eq!(resolver.resolve("llama-8b"), "llama-3.1-8b-instant");
        assert_eq!(resolver.resolve("mixtral"), "mixtral-8x7b-32768");
    }

    #[test]
    fn capabilities_and_context_follow_metadata() {
        let resolver = GroqModelResolver::new();

        for meta in GROQ_MODEL_META {
            assert!(resolver.has_capability(meta.id, Capability::CHAT));
            assert!(resolver.has_capability(meta.id, Capability::STREAMING));
            assert_eq!(
                resolver.has_capability(meta.id, Capability::TOOLS),
                meta.supports_tools,
                "model {} tool capability mismatch",
                meta.id
            );
            assert_eq!(resolver.max_context(meta.id), meta.context);
        }
    }
}
