use crate::model::{Capability, DefaultModelResolver, ModelResolver};

/// BaiLing (百灵) 模型解析器
///
/// 蚂蚁集���百灵大模型，兼容 OpenAI 协议。
///
/// ## 别名
///
/// | 别名 | 解析为 |
/// |------|--------|
/// | `bailing` / `ling` | `Ling-1T` |
/// | `ling-2.5` | `Ling-2.5-1T` |
/// | `ling-flash` / `flash` | `Ling-flash` |
/// | `ling-mini` / `mini` | `Ling-mini` |
///
/// ## 模型能力
///
/// 百灵系列模型支持多模态（VISION），可处理图片理解任务。
pub struct BaiLingModelResolver {
    inner: DefaultModelResolver,
}

impl BaiLingModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // === 模型别名 ===
        inner.extend_aliases(vec![
            ("bailing", "Ling-1T"),
            ("ling", "Ling-1T"),
            ("ling-2.5", "Ling-2.5-1T"),
            ("ling-flash", "Ling-flash"),
            ("flash", "Ling-flash"),
            ("ling-mini", "Ling-mini"),
            ("mini", "Ling-mini"),
        ]);

        // === 能力配置 ===
        // 百灵系列模型支持多模态（VISION）
        let standard_caps = Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING;
        let mini_caps = Capability::CHAT | Capability::TOOLS | Capability::STREAMING;

        inner.extend_capabilities(vec![
            ("Ling-1T", standard_caps),
            ("Ling-2.5-1T", standard_caps),
            ("Ling-flash", standard_caps),
            ("Ling-mini", mini_caps),
        ]);

        // === 上下文长度 ===
        inner.extend_context_lengths(vec![
            ("Ling-1T", 128_000),
            ("Ling-2.5-1T", 128_000),
            ("Ling-flash", 128_000),
            ("Ling-mini", 32_000),
        ]);

        Self { inner }
    }
}

impl Default for BaiLingModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for BaiLingModelResolver {
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
