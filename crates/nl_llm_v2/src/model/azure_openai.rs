use crate::model::{Capability, DefaultModelResolver, ModelResolver};

/// Azure OpenAI 模型解析器
///
/// > **注意**: Azure OpenAI 中 "模型名" 实际上是 deployment name。
/// > 用户需要将 deployment name 作为模型名传入。
///
/// ## 别名
///
/// | 别名 | 解析为 |
/// |------|--------|
/// | `azure` / `4o` | `gpt-4o` |
/// | `4o-mini` | `gpt-4o-mini` |
/// | `4.1` | `gpt-4.1` |
/// | `4.1-mini` | `gpt-4.1-mini` |
/// | `o3` | `o3-mini` |
///
/// ## 能力
///
/// - `CHAT`: 全部模型
/// - `TOOLS`: GPT 系列
/// - `STREAMING`: 全部模型
/// - `VISION`: GPT 系列
/// - `THINKING`: o3-mini
///
/// ## 上下文长度
///
/// | 模型 | 上下文 |
/// |------|--------|
/// | `gpt-4o` / `gpt-4o-mini` | 128K |
/// | `gpt-4.1` / `gpt-4.1-mini` | 1M |
/// | `o3-mini` | 200K |
pub struct AzureOpenAiModelResolver {
    inner: DefaultModelResolver,
}

impl AzureOpenAiModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // Azure 的 deployment name 由用户自定义
        // 这里只提供常见的默认名称作为别名
        inner.extend_aliases(vec![
            ("azure", "gpt-4o"),
            ("4o", "gpt-4o"),
            ("4o-mini", "gpt-4o-mini"),
            ("4.1", "gpt-4.1"),
            ("4.1-mini", "gpt-4.1-mini"),
            ("o3", "o3-mini"),
        ]);

        let gpt_caps = Capability::CHAT | Capability::TOOLS | Capability::STREAMING | Capability::VISION;
        let reasoning = Capability::CHAT | Capability::STREAMING | Capability::THINKING;

        inner.extend_capabilities(vec![
            ("gpt-4o", gpt_caps), ("gpt-4o-mini", gpt_caps),
            ("gpt-4.1", gpt_caps), ("gpt-4.1-mini", gpt_caps),
            ("o3-mini", reasoning),
        ]);

        inner.extend_context_lengths(vec![
            ("gpt-4o", 128_000), ("gpt-4o-mini", 128_000),
            ("gpt-4.1", 1_000_000), ("gpt-4.1-mini", 1_000_000),
            ("o3-mini", 200_000),
        ]);

        Self { inner }
    }
}

impl Default for AzureOpenAiModelResolver {
    fn default() -> Self { Self::new() }
}

impl ModelResolver for AzureOpenAiModelResolver {
    fn resolve(&self, model: &str) -> String { self.inner.resolve(model) }
    fn has_capability(&self, model: &str, cap: Capability) -> bool { self.inner.has_capability(model, cap) }
    fn max_context(&self, model: &str) -> usize { self.inner.max_context(model) }
    fn context_window_hint(&self, model: &str) -> (usize, usize) { self.inner.context_window_hint(model) }
}
