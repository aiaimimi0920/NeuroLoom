//! Cerebras 模型解析器
//!
//! Cerebras 提供 OpenAI 兼容接口，当前主推 Llama 3.x 系列推理模型。

use crate::model::{Capability, DefaultModelResolver, ModelResolver};

#[derive(Debug, Clone, Copy)]
pub struct CerebrasModelMeta {
    pub id: &'static str,
    pub context: usize,
    pub supports_tools: bool,
}

pub const CEREBRAS_MODEL_META: &[CerebrasModelMeta] = &[
    CerebrasModelMeta {
        id: "llama3.1-8b",
        context: 128_000,
        supports_tools: true,
    },
    CerebrasModelMeta {
        id: "llama-3.3-70b",
        context: 128_000,
        supports_tools: true,
    },
];

fn capability_for(meta: &CerebrasModelMeta) -> Capability {
    let mut caps = Capability::CHAT | Capability::STREAMING;
    if meta.supports_tools {
        caps |= Capability::TOOLS;
    }
    caps
}

pub struct CerebrasModelResolver {
    inner: DefaultModelResolver,
}

impl CerebrasModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        inner.extend_aliases(vec![
            ("cerebras", "llama3.1-8b"),
            ("llama-8b", "llama3.1-8b"),
            ("llama-70b", "llama-3.3-70b"),
            ("llama3.3-70b", "llama-3.3-70b"),
        ]);

        inner.extend_capabilities(
            CEREBRAS_MODEL_META
                .iter()
                .map(|meta| (meta.id, capability_for(meta)))
                .collect(),
        );

        inner.extend_context_lengths(
            CEREBRAS_MODEL_META
                .iter()
                .map(|meta| (meta.id, meta.context))
                .collect(),
        );

        Self { inner }
    }
}

impl Default for CerebrasModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for CerebrasModelResolver {
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
        let resolver = CerebrasModelResolver::new();
        assert_eq!(resolver.resolve("cerebras"), "llama3.1-8b");
        assert_eq!(resolver.resolve("llama3.3-70b"), "llama-3.3-70b");
    }

    #[test]
    fn capabilities_and_context_follow_metadata() {
        let resolver = CerebrasModelResolver::new();

        for meta in CEREBRAS_MODEL_META {
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
