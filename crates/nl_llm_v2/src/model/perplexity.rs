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

        let search_caps = Capability::CHAT | Capability::STREAMING;
        let reasoning_caps = search_caps | Capability::THINKING;

        inner.extend_capabilities(vec![
            ("sonar-pro", search_caps),
            ("sonar", search_caps),
            ("sonar-reasoning-pro", reasoning_caps),
            ("sonar-reasoning", reasoning_caps),
            ("sonar-deep-research", search_caps),
            ("r1-1776", reasoning_caps),
        ]);

        inner.extend_context_lengths(vec![
            ("sonar-pro", 200_000),
            ("sonar", 128_000),
            ("sonar-reasoning-pro", 128_000),
            ("sonar-reasoning", 128_000),
            ("sonar-deep-research", 128_000),
            ("r1-1776", 128_000),
        ]);

        Self { inner }
    }
}

impl Default for PerplexityModelResolver {
    fn default() -> Self { Self::new() }
}

impl ModelResolver for PerplexityModelResolver {
    fn resolve(&self, model: &str) -> String { self.inner.resolve(model) }
    fn has_capability(&self, model: &str, cap: Capability) -> bool { self.inner.has_capability(model, cap) }
    fn max_context(&self, model: &str) -> usize { self.inner.max_context(model) }
    fn context_window_hint(&self, model: &str) -> (usize, usize) { self.inner.context_window_hint(model) }
}
