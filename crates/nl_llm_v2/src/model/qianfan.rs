//! 百度千帆大模型平台模型解析器
//!
//! ## 模型别名
//!
//! | 别名 | 解析为 |
//! |------|--------|
//! | `qianfan` / `ernie` / `文心` | ernie-4.5-turbo-128k |
//! | `4.5` | ernie-4.5-turbo-128k |
//! | `4.0` | ernie-4.0-turbo-128k |
//! | `3.5` | ernie-3.5-128k |
//! | `speed` | ernie-speed-128k |
//! | `lite` | ernie-lite-128k |
//! | `tiny` | ernie-tiny-8k |

use crate::model::{Capability, DefaultModelResolver, ModelResolver};

/// 百度千帆模型解析器
pub struct QianfanModelResolver {
    inner: DefaultModelResolver,
}

impl QianfanModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        inner.extend_aliases(vec![
            ("qianfan", "ernie-4.5-turbo-128k"),
            ("ernie", "ernie-4.5-turbo-128k"),
            ("文心", "ernie-4.5-turbo-128k"),
            ("4.5", "ernie-4.5-turbo-128k"),
            ("4.0", "ernie-4.0-turbo-128k"),
            ("3.5", "ernie-3.5-128k"),
            ("speed", "ernie-speed-128k"),
            ("lite", "ernie-lite-128k"),
            ("tiny", "ernie-tiny-8k"),
        ]);

        let ernie_caps = Capability::CHAT | Capability::TOOLS | Capability::STREAMING;
        let ernie_vision = ernie_caps | Capability::VISION;

        inner.extend_capabilities(vec![
            ("ernie-4.5-turbo-128k", ernie_vision),
            ("ernie-4.5-8k", ernie_vision),
            ("ernie-4.0-turbo-128k", ernie_caps),
            ("ernie-4.0-turbo-8k", ernie_caps),
            ("ernie-3.5-128k", ernie_caps),
            ("ernie-3.5-8k", ernie_caps),
            ("ernie-speed-128k", ernie_caps),
            ("ernie-speed-8k", ernie_caps),
            ("ernie-lite-128k", ernie_caps),
            ("ernie-lite-8k", ernie_caps),
            ("ernie-tiny-8k", ernie_caps),
        ]);

        inner.extend_context_lengths(vec![
            ("ernie-4.5-turbo-128k", 128_000),
            ("ernie-4.5-8k", 8_000),
            ("ernie-4.0-turbo-128k", 128_000),
            ("ernie-4.0-turbo-8k", 8_000),
            ("ernie-3.5-128k", 128_000),
            ("ernie-3.5-8k", 8_000),
            ("ernie-speed-128k", 128_000),
            ("ernie-speed-8k", 8_000),
            ("ernie-lite-128k", 128_000),
            ("ernie-lite-8k", 8_000),
            ("ernie-tiny-8k", 8_000),
        ]);

        Self { inner }
    }
}

impl Default for QianfanModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for QianfanModelResolver {
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
