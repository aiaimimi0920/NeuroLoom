use crate::model::{Capability, DefaultModelResolver, ModelResolver};

/// MiniMax 模型解析器
///
/// ## 支持的模型
///
/// | 模型 | 上下文 | 说明 |
/// |------|--------|------|
/// | `MiniMax-M2.5` | 200K | 旗舰模型，支持 CoT 思考 |
/// | `MiniMax-M2.5-highspeed` | 200K | 旗舰高速版 |
/// | `MiniMax-M2.1` | 200K | 编程增强版，支持 CoT 思考 |
/// | `MiniMax-M2.1-highspeed` | 200K | 编程增强高速版 |
/// | `MiniMax-M2` | 128K | 标准模型 |
/// | `M2-her` | 128K | 多角色扮演模型 |
///
/// ## 别名
///
/// | 别名 | 解析为 |
/// |------|--------|
/// | `minimax` / `m2.5` | `MiniMax-M2.5` |
/// | `m2.5-fast` | `MiniMax-M2.5-highspeed` |
/// | `m2.1` | `MiniMax-M2.1` |
/// | `m2.1-fast` | `MiniMax-M2.1-highspeed` |
/// | `m2` | `MiniMax-M2` |
/// | `her` | `M2-her` |
///
/// ## 能力
///
/// - `CHAT`: 支持对话
/// - `TOOLS`: 支持工具调用
/// - `STREAMING`: 支持流式输出
/// - `THINKING`: 支持 Chain of Thought 思考 (M2.5, M2.1)
pub struct MiniMaxModelResolver {
    inner: DefaultModelResolver,
}

impl MiniMaxModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // === 模型别名 ===
        inner.extend_aliases(vec![
            ("minimax", "MiniMax-M2.5"),
            ("m2.5", "MiniMax-M2.5"),
            ("m2.5-fast", "MiniMax-M2.5-highspeed"),
            ("m2.1", "MiniMax-M2.1"),
            ("m2.1-fast", "MiniMax-M2.1-highspeed"),
            ("m2", "MiniMax-M2"),
            ("her", "M2-her"),
        ]);

        // === 能力配置 ===
        // M2.5 和 M2.1 支持 Chain of Thought (CoT) 思考
        let thinking_caps = Capability::CHAT | Capability::TOOLS | Capability::STREAMING | Capability::THINKING;
        let standard_caps = Capability::CHAT | Capability::TOOLS | Capability::STREAMING;

        inner.extend_capabilities(vec![
            // M2.5 系列 - 旗舰模型
            ("MiniMax-M2.5", thinking_caps),
            ("MiniMax-M2.5-highspeed", standard_caps),
            // M2.1 系列 - 编程增强版
            ("MiniMax-M2.1", thinking_caps),
            ("MiniMax-M2.1-highspeed", standard_caps),
            // M2 系列 - 标准模型
            ("MiniMax-M2", standard_caps),
            // M2-her - 多角色扮演
            ("M2-her", standard_caps),
        ]);

        // === 上下文长度 ===
        inner.extend_context_lengths(vec![
            // M2.5 系列
            ("MiniMax-M2.5", 200_000),
            ("MiniMax-M2.5-highspeed", 200_000),
            // M2.1 系列
            ("MiniMax-M2.1", 200_000),
            ("MiniMax-M2.1-highspeed", 200_000),
            // M2 系列
            ("MiniMax-M2", 128_000),
            // M2-her
            ("M2-her", 128_000),
        ]);

        Self { inner }
    }
}

impl Default for MiniMaxModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for MiniMaxModelResolver {
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
