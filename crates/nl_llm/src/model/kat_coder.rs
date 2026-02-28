use super::default::DefaultModelResolver;
use super::resolver::{Capability, ModelResolver};

/// KAT-Coder (StreamLake) 平台专属模型解析器
///
/// 快手旗下 StreamLake 平台推出的代码大模型服务，通过 Vanchin 网关提供 API。
///
/// ## 支持的模型
///
/// | 模型 | 能力 | 上下文 | 说明 |
/// |------|------|--------|------|
/// | `KAT-Coder-Pro` | Chat, Tools, Streaming | 128K | 旗舰代码模型 |
/// | `KAT-Coder-Pro-V1` | Chat, Tools, Streaming | 128K | 旗舰版本 V1（Claude 代理模式）|
/// | `KAT-Coder-Air-V1` | Chat, Tools, Streaming | 128K | 轻量级代码模型 |
///
/// ## 常用别名
///
/// | 别名 | 解析为 | 说明 |
/// |------|--------|------|
/// | `kat_coder` / `pro` | `KAT-Coder-Pro` | 默认旗舰模型 |
/// | `pro-v1` | `KAT-Coder-Pro-V1` | 旗舰版本 V1 |
/// | `air` / `air-v1` | `KAT-Coder-Air-V1` | 轻量快速模型 |
///
/// ## 示例
///
/// ```rust
/// use nl_llm::model::kat_coder::KatCoderModelResolver;
/// use nl_llm::model::resolver::Capability;
///
/// let resolver = KatCoderModelResolver::new();
///
/// // 别名解析
/// assert_eq!(resolver.resolve("pro"), "kat-coder-pro");
/// assert_eq!(resolver.resolve("air"), "kat-coder-air-v1");
///
/// // 能力检测
/// assert!(resolver.has_capability("kat-coder-pro", Capability::CHAT));
/// assert!(resolver.has_capability("kat-coder-pro", Capability::TOOLS));
/// assert!(resolver.has_capability("kat-coder-pro", Capability::STREAMING));
///
/// // 上下文长度
/// assert_eq!(resolver.max_context("kat-coder-pro"), 128_000);
/// assert_eq!(resolver.max_context("air"), 128_000);  // 别名也能解析
/// ```
pub struct KatCoderModelResolver {
    inner: DefaultModelResolver,
}

impl KatCoderModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // ========== 模型别名 ==========
        inner.extend_aliases(vec![
            ("kat_coder", "kat-coder-pro"),
            ("katcoder", "kat-coder-pro"),
            ("pro", "kat-coder-pro"),
            ("pro-v1", "kat-coder-pro-v1"),
            ("air", "kat-coder-air-v1"),
            ("air-v1", "kat-coder-air-v1"),
        ]);

        // ========== 能力配置 ==========
        let standard_caps = Capability::CHAT | Capability::TOOLS | Capability::STREAMING;

        inner.extend_capabilities(vec![
            ("kat-coder-pro", standard_caps),
            ("kat-coder-pro-v1", standard_caps),
            ("kat-coder-air-v1", standard_caps),
        ]);

        // ========== 上下文长度 ==========
        inner.extend_context_lengths(vec![
            ("kat-coder-pro", 128_000),
            ("kat-coder-pro-v1", 128_000),
            ("kat-coder-air-v1", 128_000),
        ]);

        Self { inner }
    }
}

impl Default for KatCoderModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for KatCoderModelResolver {
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
