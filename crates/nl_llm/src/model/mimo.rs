use crate::model::{Capability, DefaultModelResolver, ModelResolver};

/// Xiaomi MiMo 模型解析器
///
/// ## 支持的模型
///
/// | 模型 | 上下文 | 说明 |
/// |------|--------|------|
/// | `mimo-v2-flash` | 128K | 旗舰模型，支持思考模式 |
///
/// ## 别名
///
/// | 别名 | 解析为 |
/// |------|--------|
/// | `mimo` / `flash` | `mimo-v2-flash` |
///
/// ## 能力
///
/// - `CHAT`: 支持对话
/// - `TOOLS`: 支持工具调用
/// - `STREAMING`: 支持流式输出
/// - `THINKING`: 支持思考模式（Chain of Thought）
///
/// ## 使用示例
///
/// ```rust
/// use nl_llm::model::mimo::MiMoModelResolver;
/// use nl_llm::model::{ModelResolver, Capability};
///
/// let resolver = MiMoModelResolver::new();
///
/// // 别名解析
/// assert_eq!(resolver.resolve("mimo"), "mimo-v2-flash");
/// assert_eq!(resolver.resolve("flash"), "mimo-v2-flash");
///
/// // 能力检测
/// assert!(resolver.has_capability("mimo-v2-flash", Capability::CHAT));
/// assert!(resolver.has_capability("mimo-v2-flash", Capability::THINKING));
///
/// // 上下文长度
/// assert_eq!(resolver.max_context("mimo-v2-flash"), 128_000);
/// ```
pub struct MiMoModelResolver {
    inner: DefaultModelResolver,
}

impl MiMoModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        inner.extend_aliases(vec![("mimo", "mimo-v2-flash"), ("flash", "mimo-v2-flash")]);

        let full_caps =
            Capability::CHAT | Capability::TOOLS | Capability::STREAMING | Capability::THINKING;
        inner.extend_capabilities(vec![("mimo-v2-flash", full_caps)]);

        inner.extend_context_lengths(vec![("mimo-v2-flash", 128_000)]);

        Self { inner }
    }
}

impl Default for MiMoModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for MiMoModelResolver {
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
