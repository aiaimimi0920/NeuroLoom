use crate::model::default::DefaultModelResolver;
use crate::model::resolver::{ModelResolver, Capability};

/// Sourcegraph Amp 模型解析器
///
/// Amp 平台聚合了多个后端供应商（OpenAI / Anthropic / Google Gemini 等），
/// 因此模型别名和能力列表覆盖所有主流平台的模型。
///
/// ## 特有别名
///
/// | 别名 | 解析为 | 说明 |
/// |------|--------|------|
/// | `best` | `gemini-2.5-pro` | 最强能力模型 |
/// | `fast` | `gemini-2.5-flash` | 快速响应模型 |
/// | `cheap` | `gpt-4o-mini` | 低成本模型 |
/// | `reasoning` | `o1` | 推理模型 |
///
/// ## 模型上下文长度
///
/// | 模型 | 上下文长度 |
/// |------|-----------|
/// | GPT-4o 系列 | 128K |
/// | o1 / o1-mini / o3-mini | 200K |
/// | Claude 系列 | 200K |
/// | Gemini 系列 | 1M |
///
/// ## 示例
///
/// ```
/// let resolver = AmpModelResolver::new();
///
/// // 使用便捷别名
/// assert_eq!(resolver.resolve("best"), "gemini-2.5-pro");
/// assert_eq!(resolver.resolve("fast"), "gemini-2.5-flash");
///
/// // 检查能力
/// assert!(resolver.has_capability("gpt-4o", Capability::VISION));
/// assert!(resolver.has_capability("o1", Capability::CHAT));
/// assert!(!resolver.has_capability("o1", Capability::STREAMING)); // o1 不支持流式
///
/// // 获取上下文长度
/// assert_eq!(resolver.max_context("gpt-4o"), 128_000);
/// assert_eq!(resolver.max_context("gemini-2.5-pro"), 1_000_000);
/// ```
pub struct AmpModelResolver {
    inner: DefaultModelResolver,
}

impl AmpModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // ========== 别名配置 ==========
        // Amp 平台特有的跨平台便捷别名
        inner.set_alias("best", "gemini-2.5-pro");
        inner.set_alias("fast", "gemini-2.5-flash");
        inner.set_alias("cheap", "gpt-4o-mini");

        // o1 系列别名
        inner.set_alias("reasoning", "o1");
        inner.set_alias("reasoning-mini", "o1-mini");

        // Claude 简化别名
        inner.set_alias("claude", "claude-sonnet-4-20250514");
        inner.set_alias("claude-opus", "claude-opus-4-20250514");
        inner.set_alias("claude-sonnet", "claude-sonnet-4-20250514");

        // Gemini 简化别名
        inner.set_alias("gemini", "gemini-2.5-flash");
        inner.set_alias("gemini-pro", "gemini-2.5-pro");

        // ========== 能力配置 ==========
        // GPT-4o 系列：支持 Chat、Vision、Tools、Streaming
        inner.set_capability("gpt-4o", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING);
        inner.set_capability("gpt-4o-mini", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING);

        // o1 系列：
        // - o1: 支持 Chat，不支持 Streaming 和 Vision
        // - o1-mini: 支持 Chat，不支持 Streaming 和 Vision
        // - o3-mini: 支持 Chat 和 Streaming，不支持 Vision
        inner.set_capability("o1", Capability::CHAT);
        inner.set_capability("o1-mini", Capability::CHAT);
        inner.set_capability("o3-mini", Capability::CHAT | Capability::STREAMING);

        // Claude 系列：支持 Chat、Vision、Tools、Streaming
        inner.set_capability("claude-sonnet-4-20250514", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING);
        inner.set_capability("claude-opus-4-20250514", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING);

        // Gemini 系列：支持全部能力（包括 Thinking）
        inner.set_capability("gemini-2.5-pro", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING | Capability::THINKING);
        inner.set_capability("gemini-2.5-flash", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING | Capability::THINKING);

        // ========== 上下文长度配置 ==========
        // GPT-4o 系列: 128K
        inner.set_context_length("gpt-4o", 128_000);
        inner.set_context_length("gpt-4o-mini", 128_000);

        // o1 系列: 200K
        inner.set_context_length("o1", 200_000);
        inner.set_context_length("o1-mini", 200_000);
        inner.set_context_length("o3-mini", 200_000);

        // Claude 系列: 200K
        inner.set_context_length("claude-sonnet-4-20250514", 200_000);
        inner.set_context_length("claude-opus-4-20250514", 200_000);

        // Gemini 系列: 1M
        inner.set_context_length("gemini-2.5-pro", 1_000_000);
        inner.set_context_length("gemini-2.5-flash", 1_000_000);

        Self { inner }
    }
}

impl Default for AmpModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for AmpModelResolver {
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
    fn intelligence_and_modality(&self, _model: &str) -> Option<(f32, crate::model::resolver::Modality)> { None }

}
