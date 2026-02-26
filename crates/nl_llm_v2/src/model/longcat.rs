use crate::model::{Capability, DefaultModelResolver, ModelResolver};

/// Longcat AI 模型解析器
///
/// 处理模型别名映射和能力配置。
///
/// ## 支持的模型
///
/// | 模型 | 能力 | 上下文 | 说明 |
/// |------|------|--------|------|
/// | `LongCat-Flash-Chat` | Chat, Tools, Streaming | 128K | 基础模型 |
///
/// ## 常用别名
///
/// | 别名 | 解析为 | 说明 |
/// |------|--------|------|
/// | `longcat` / `flash` | `LongCat-Flash-Chat` | 默认模型 |
///
/// ## 示例
///
/// ```rust
/// use nl_llm_v2::model::longcat::LongcatModelResolver;
/// use nl_llm_v2::model::resolver::Capability;
///
/// let resolver = LongcatModelResolver::new();
///
/// // 别名解析
/// assert_eq!(resolver.resolve("longcat"), "LongCat-Flash-Chat");
/// assert_eq!(resolver.resolve("flash"), "LongCat-Flash-Chat");
///
/// // 能力检测
/// assert!(resolver.has_capability("LongCat-Flash-Chat", Capability::CHAT));
/// assert!(resolver.has_capability("LongCat-Flash-Chat", Capability::TOOLS));
/// assert!(resolver.has_capability("LongCat-Flash-Chat", Capability::STREAMING));
///
/// // 上下文长度
/// assert_eq!(resolver.max_context("LongCat-Flash-Chat"), 128_000);
/// assert_eq!(resolver.max_context("flash"), 128_000);  // 别名也能解析
/// ```
pub struct LongcatModelResolver {
    inner: DefaultModelResolver,
}

impl LongcatModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // ========== 模型别名 ==========
        inner.extend_aliases(vec![
            ("longcat", "LongCat-Flash-Chat"),
            ("flash", "LongCat-Flash-Chat"),
        ]);

        // ========== 能力配置 ==========
        // 目前先假定具备基本的一家通用的流式和工具支持，后续根据平台特性可增减
        let standard_caps = Capability::CHAT | Capability::TOOLS | Capability::STREAMING;

        inner.extend_capabilities(vec![("LongCat-Flash-Chat", standard_caps)]);

        // ========== 上下文长度 ==========
        // 暂无确切数据，先保守设置 128K
        inner.extend_context_lengths(vec![("LongCat-Flash-Chat", 128_000)]);

        Self { inner }
    }
}

impl Default for LongcatModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for LongcatModelResolver {
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
