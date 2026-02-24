use super::default::DefaultModelResolver;
use super::resolver::{ModelResolver, Capability};

/// DeepSeek 模型解析器
///
/// DeepSeek 提供两类模型：
/// - **deepseek-chat**: DeepSeek-V3.2 通用对话模型（非推理）
/// - **deepseek-reasoner**: DeepSeek-V3.2 推理模型（链式思考）
///
/// ## 模型别名
///
/// | 别名 | 解析为 | 说明 |
/// |------|--------|------|
/// | `deepseek` | `deepseek-chat` | 默认对话模型 |
/// | `ds` | `deepseek-chat` | 简写 |
/// | `chat` | `deepseek-chat` | 对话模型 |
/// | `reasoner` | `deepseek-reasoner` | 推理模型 |
/// | `r1` | `deepseek-reasoner` | R1 系列推理 |
/// | `think` | `deepseek-reasoner` | 思考模式 |
///
/// ## 模型能力
///
/// | 模型 | CHAT | TOOLS | STREAMING | THINKING |
/// |------|------|-------|-----------|----------|
/// | deepseek-chat | ✅ | ✅ | ✅ | ❌ |
/// | deepseek-reasoner | ✅ | ❌ | ✅ | ✅ |
///
/// ## 上下文长度
///
/// 所有模型均为 64K 上下文。
///
/// ## 示例
///
/// ```rust
/// use nl_llm_v2::model::deepseek::DeepSeekModelResolver;
/// use nl_llm_v2::model::resolver::Capability;
///
/// let resolver = DeepSeekModelResolver::new();
///
/// // 别名解析
/// assert_eq!(resolver.resolve("ds"), "deepseek-chat");
/// assert_eq!(resolver.resolve("r1"), "deepseek-reasoner");
/// assert_eq!(resolver.resolve("think"), "deepseek-reasoner");
///
/// // 能力检测
/// assert!(resolver.has_capability("deepseek-chat", Capability::TOOLS));
/// assert!(!resolver.has_capability("deepseek-reasoner", Capability::TOOLS));
/// assert!(resolver.has_capability("deepseek-reasoner", Capability::THINKING));
///
/// // 上下文长度
/// assert_eq!(resolver.max_context("deepseek-chat"), 64_000);
/// ```
pub struct DeepSeekModelResolver {
    inner: DefaultModelResolver,
}

impl DeepSeekModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // ========== 模型别名 ==========
        inner.extend_aliases(vec![
            ("deepseek", "deepseek-chat"),
            ("ds", "deepseek-chat"),
            ("chat", "deepseek-chat"),
            ("reasoner", "deepseek-reasoner"),
            ("r1", "deepseek-reasoner"),
            ("think", "deepseek-reasoner"),
        ]);

        // ========== 能力配置 ==========
        // deepseek-chat: 支持 Chat、Tools、Streaming，不支持 Thinking
        // deepseek-reasoner: 支持 Chat、Streaming、Thinking，不支持 Tools
        inner.extend_capabilities(vec![
            ("deepseek-chat", Capability::CHAT | Capability::TOOLS | Capability::STREAMING),
            ("deepseek-reasoner", Capability::CHAT | Capability::STREAMING | Capability::THINKING),
        ]);

        // ========== 上下文长度 ==========
        // DeepSeek V3.2 系列均为 64K 上下文
        inner.extend_context_lengths(vec![
            ("deepseek-chat", 64_000),
            ("deepseek-reasoner", 64_000),
        ]);

        Self { inner }
    }
}

impl Default for DeepSeekModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for DeepSeekModelResolver {
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
}
