use crate::model::default::DefaultModelResolver;
use crate::model::resolver::{Capability, ModelResolver};

/// 百川智能平台专属模型解析器
pub struct BaichuanModelResolver {
    inner: DefaultModelResolver,
}

impl BaichuanModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        inner.extend_aliases(vec![
            ("baichuan4", "Baichuan4"),
            ("baichuan4-turbo", "Baichuan4-Turbo"),
            ("baichuan4-air", "Baichuan4-Air"),
            ("baichuan3-turbo", "Baichuan3-Turbo"),
            ("baichuan3-turbo-128k", "Baichuan3-Turbo-128k"),
            ("baichuan2-turbo", "Baichuan2-Turbo"),
            ("baichuan2-turbo-192k", "Baichuan2-Turbo-192k"),
            ("bc4", "Baichuan4"),
            ("bc4-air", "Baichuan4-Air"),
        ]);

        let standard_caps = Capability::CHAT | Capability::STREAMING | Capability::TOOLS;

        // 配置百川智能的主流模型别名和能力
        // 参考百川 API 接入文档
        inner.extend_capabilities(vec![
            // 旗舰模型
            ("Baichuan4", standard_caps),
            ("Baichuan4-Turbo", standard_caps),
            ("Baichuan3-Turbo", standard_caps),
            ("Baichuan3-Turbo-128k", standard_caps),
            // 极速/低成本版本
            ("Baichuan4-Air", standard_caps),
            ("Baichuan2-Turbo", standard_caps),
            ("Baichuan2-Turbo-192k", standard_caps),
        ]);

        // 配置上下文长度
        inner.extend_context_lengths(vec![
            ("Baichuan4", 32_000), // 假设基础长度为 32k，视情况调整
            ("Baichuan4-Turbo", 32_000),
            ("Baichuan3-Turbo", 32_000),
            ("Baichuan3-Turbo-128k", 128_000),
            ("Baichuan4-Air", 32_000),
            ("Baichuan2-Turbo", 32_000),
            ("Baichuan2-Turbo-192k", 192_000),
        ]);

        Self { inner }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_common_aliases() {
        let resolver = BaichuanModelResolver::new();
        assert_eq!(resolver.resolve("baichuan4-air"), "Baichuan4-Air");
        assert_eq!(resolver.resolve("bc4"), "Baichuan4");
    }

    #[test]
    fn provides_context_and_capabilities() {
        let resolver = BaichuanModelResolver::new();
        assert_eq!(resolver.max_context("Baichuan3-Turbo-128k"), 128_000);
        assert!(resolver.has_capability("Baichuan4", Capability::STREAMING));
        assert!(resolver.has_capability("baichuan2-turbo", Capability::TOOLS));
    }
}

impl Default for BaichuanModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for BaichuanModelResolver {
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
        model: &str,
    ) -> Option<(f32, crate::model::resolver::Modality)> {
        self.inner.intelligence_and_modality(model)
    }
}
