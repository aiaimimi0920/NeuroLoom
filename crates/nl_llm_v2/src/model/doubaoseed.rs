use crate::model::{Capability, DefaultModelResolver, ModelResolver};

/// DouBaoSeed (字节跳动 · 豆包) 模型解析器
///
/// 火山引擎 ARK 平台提供的豆包系列模型。
///
/// ## 别名
///
/// | 别名 | 解析为 |
/// |------|--------|
/// | `doubao` / `seed` / `pro` | `doubao-seed-2-0-pro-260215` |
/// | `doubao-code` / `code` | `doubao-seed-2-0-code-preview-latest` |
/// | `doubao-lite` / `lite` | `doubao-seed-1-6-lite-250115` |
/// | `doubao-32k` | `doubao-pro-32k-241215` |
/// | `doubao-128k` | `doubao-pro-128k-241215` |
pub struct DouBaoSeedModelResolver {
    inner: DefaultModelResolver,
}

impl DouBaoSeedModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // === 模型别名 ===
        inner.extend_aliases(vec![
            // Seed 2.0 系列
            ("doubao", "doubao-seed-2-0-pro-260215"),
            ("seed", "doubao-seed-2-0-pro-260215"),
            ("pro", "doubao-seed-2-0-pro-260215"),
            ("doubao-code", "doubao-seed-2-0-code-preview-latest"),
            ("code", "doubao-seed-2-0-code-preview-latest"),
            // Seed 1.6 系列
            ("doubao-lite", "doubao-seed-1-6-lite-250115"),
            ("lite", "doubao-seed-1-6-lite-250115"),
            // Pro 系列（旧版命名）
            ("doubao-32k", "doubao-pro-32k-241215"),
            ("doubao-128k", "doubao-pro-128k-241215"),
        ]);

        // === 能力配置 ===
        // 豆包系列模型支持多模态（VISION）
        let standard_caps = Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING;
        let code_caps = Capability::CHAT | Capability::TOOLS | Capability::STREAMING;
        let thinking_caps = Capability::CHAT | Capability::STREAMING | Capability::THINKING;

        inner.extend_capabilities(vec![
            // Seed 2.0 系列
            ("doubao-seed-2-0-pro-260215", standard_caps),
            ("doubao-seed-2-0-code-preview-latest", code_caps),
            // Seed 1.6 系列
            ("doubao-seed-1-6-lite-250115", standard_caps),
            // Pro 系列
            ("doubao-pro-32k-241215", standard_caps),
            ("doubao-pro-128k-241215", standard_caps),
            // 思考模型
            ("doubao-1-5-thinking-pro-250415", thinking_caps),
        ]);

        // === 上下文长度 ===
        inner.extend_context_lengths(vec![
            // Seed 2.0 系列
            ("doubao-seed-2-0-pro-260215", 128_000),
            ("doubao-seed-2-0-code-preview-latest", 128_000),
            // Seed 1.6 系列
            ("doubao-seed-1-6-lite-250115", 64_000),
            // Pro 系列
            ("doubao-pro-32k-241215", 32_000),
            ("doubao-pro-128k-241215", 128_000),
            // 思考模型
            ("doubao-1-5-thinking-pro-250415", 128_000),
        ]);

        Self { inner }
    }
}

impl Default for DouBaoSeedModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for DouBaoSeedModelResolver {
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
