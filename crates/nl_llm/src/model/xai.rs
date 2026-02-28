use super::default::DefaultModelResolver;
use super::resolver::{Capability, ModelResolver};

#[derive(Debug, Clone, Copy)]
pub struct XaiModelMeta {
    pub id: &'static str,
    pub context: usize,
    pub vision: bool,
}

pub const XAI_MODEL_META: &[XaiModelMeta] = &[
    XaiModelMeta {
        id: "grok-4-latest",
        context: 131_072,
        vision: false,
    },
    XaiModelMeta {
        id: "grok-3-latest",
        context: 131_072,
        vision: false,
    },
    XaiModelMeta {
        id: "grok-3-mini",
        context: 131_072,
        vision: false,
    },
    XaiModelMeta {
        id: "grok-2-latest",
        context: 131_072,
        vision: false,
    },
    XaiModelMeta {
        id: "grok-vision-latest",
        context: 8_192,
        vision: true,
    },
];

fn capability_for(meta: &XaiModelMeta) -> Capability {
    let mut caps = Capability::CHAT | Capability::STREAMING | Capability::TOOLS;
    if meta.vision {
        caps |= Capability::VISION;
    }
    caps
}

/// x.AI (Grok) 模型解析器
///
/// 支持的模型（参考 x.AI API 文档）：
/// - grok-4-latest
/// - grok-3-latest
/// - grok-3-mini
/// - grok-2-latest
/// - grok-vision-latest
pub struct XaiModelResolver {
    inner: DefaultModelResolver,
}

impl XaiModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // === 模型别名 ===
        inner.extend_aliases(vec![
            ("grok", "grok-4-latest"),
            ("grok-4", "grok-4-latest"),
            ("grok-3", "grok-3-latest"),
            ("grok-2", "grok-2-latest"),
            ("grok-vision", "grok-vision-latest"),
        ]);

        // === 能力配置 ===
        inner.extend_capabilities(
            XAI_MODEL_META
                .iter()
                .map(|meta| (meta.id, capability_for(meta)))
                .collect(),
        );

        // === 上下文长度 ===
        // Note: setting context lengths based on typical Grok specs (131k context window)
        inner.extend_context_lengths(
            XAI_MODEL_META
                .iter()
                .map(|meta| (meta.id, meta.context))
                .collect(),
        );

        Self { inner }
    }
}

impl Default for XaiModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for XaiModelResolver {
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
        let resolver = XaiModelResolver::new();
        assert_eq!(resolver.resolve("grok"), "grok-4-latest");
        assert_eq!(resolver.resolve("grok-3"), "grok-3-latest");
        assert_eq!(resolver.resolve("grok-vision"), "grok-vision-latest");
    }

    #[test]
    fn capability_and_context_follow_metadata() {
        let resolver = XaiModelResolver::new();

        for meta in XAI_MODEL_META {
            assert!(resolver.has_capability(meta.id, Capability::CHAT));
            assert!(resolver.has_capability(meta.id, Capability::STREAMING));
            assert!(resolver.has_capability(meta.id, Capability::TOOLS));
            assert_eq!(
                resolver.has_capability(meta.id, Capability::VISION),
                meta.vision,
                "model {} vision capability mismatch",
                meta.id
            );
            assert_eq!(resolver.max_context(meta.id), meta.context);
        }
    }
}
