//! Perplexity AI 模型解析器
//!
//! ## 模型别名
//!
//! | 别名 | 解析为 |
//! |------|--------|
//! | `perplexity` / `pplx` / `sonar-pro` | sonar-pro |
//! | `sonar` | sonar |
//! | `reasoning` | sonar-reasoning-pro |
//! | `research` | sonar-deep-research |
//! | `r1` | r1-1776 |

use crate::model::{Capability, DefaultModelResolver, ModelResolver};

#[derive(Debug, Clone, Copy)]
pub struct PerplexityModelMeta {
    pub id: &'static str,
    pub summary: &'static str,
    pub context: usize,
    pub price_per_million: &'static str,
    pub reasoning: bool,
}

pub const PERPLEXITY_MODEL_META: &[PerplexityModelMeta] = &[
    PerplexityModelMeta {
        id: "sonar-pro",
        summary: "Sonar Pro — 旗舰搜索模型",
        context: 200_000,
        price_per_million: "$3/$15",
        reasoning: false,
    },
    PerplexityModelMeta {
        id: "sonar",
        summary: "Sonar — 标准搜索模型",
        context: 128_000,
        price_per_million: "$1/$1",
        reasoning: false,
    },
    PerplexityModelMeta {
        id: "sonar-reasoning-pro",
        summary: "Sonar Reasoning Pro — 深度推理+搜索",
        context: 128_000,
        price_per_million: "$2/$8",
        reasoning: true,
    },
    PerplexityModelMeta {
        id: "sonar-reasoning",
        summary: "Sonar Reasoning — 标准推理+搜索",
        context: 128_000,
        price_per_million: "$1/$5",
        reasoning: true,
    },
    PerplexityModelMeta {
        id: "sonar-deep-research",
        summary: "Sonar Deep Research — 深度研究",
        context: 128_000,
        price_per_million: "$2/$8",
        reasoning: false,
    },
    PerplexityModelMeta {
        id: "r1-1776",
        summary: "R1-1776 — 离线推理模型(无搜索)",
        context: 128_000,
        price_per_million: "$2/$8",
        reasoning: true,
    },
];

fn capability_for(meta: &PerplexityModelMeta) -> Capability {
    let mut caps = Capability::CHAT | Capability::STREAMING;
    if meta.reasoning {
        caps |= Capability::THINKING;
    }
    caps
}

/// Perplexity AI 模型解析器
pub struct PerplexityModelResolver {
    inner: DefaultModelResolver,
}

impl PerplexityModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        inner.extend_aliases(vec![
            ("perplexity", "sonar-pro"),
            ("pplx", "sonar-pro"),
            ("sonar-pro", "sonar-pro"),
            ("sonar", "sonar"),
            ("reasoning", "sonar-reasoning-pro"),
            ("research", "sonar-deep-research"),
            ("r1", "r1-1776"),
        ]);

        inner.extend_capabilities(
            PERPLEXITY_MODEL_META
                .iter()
                .map(|meta| (meta.id, capability_for(meta)))
                .collect(),
        );

        inner.extend_context_lengths(
            PERPLEXITY_MODEL_META
                .iter()
                .map(|meta| (meta.id, meta.context))
                .collect(),
        );

        Self { inner }
    }
}

impl Default for PerplexityModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for PerplexityModelResolver {
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
        let resolver = PerplexityModelResolver::new();
        assert_eq!(resolver.resolve("pplx"), "sonar-pro");
        assert_eq!(resolver.resolve("reasoning"), "sonar-reasoning-pro");
        assert_eq!(resolver.resolve("research"), "sonar-deep-research");
    }

    #[test]
    fn reasoning_models_expose_thinking_capability() {
        let resolver = PerplexityModelResolver::new();
        for meta in PERPLEXITY_MODEL_META {
            assert_eq!(
                resolver.has_capability(meta.id, Capability::THINKING),
                meta.reasoning,
                "model {} thinking capability mismatch",
                meta.id
            );
        }
    }
}
