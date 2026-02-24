use super::default::DefaultModelResolver;
use super::resolver::{ModelResolver, Capability};

/// 智谱 AI (Zhipu / BigModel) 模型解析器
///
/// 支持的模型（截至 2026-02-25）：
///
/// ## 常规模型
/// - glm-5: 旗舰模型（最新）
/// - glm-4: 多模态模型
/// - glm-4-flash: 轻量快速模型
/// - glm-4-plus: 增强模型
/// - glm-4-air: 轻量模型
/// - glm-4-long: 长上下文模型（1M context）
///
/// ## 思考模型（推理增强）
/// - glm-z1-airx: 思考模型
/// - glm-z1-flash: 快速思考模型
pub struct ZhipuModelResolver {
    inner: DefaultModelResolver,
}

impl ZhipuModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // === 模型别名 ===
        inner.extend_aliases(vec![
            // 常规模型
            ("glm", "glm-5"),
            ("glm5", "glm-5"),
            ("glm-5", "glm-5"),
            ("glm4", "glm-4"),
            ("glm-4", "glm-4"),
            ("glm-flash", "glm-4-flash"),
            ("flash", "glm-4-flash"),
            ("glm-plus", "glm-4-plus"),
            ("plus", "glm-4-plus"),
            ("glm-air", "glm-4-air"),
            ("air", "glm-4-air"),
            ("glm-long", "glm-4-long"),
            ("long", "glm-4-long"),
            // 思考模型
            ("glm-z1", "glm-z1-airx"),
            ("z1", "glm-z1-airx"),
            ("z1-airx", "glm-z1-airx"),
            ("z1-flash", "glm-z1-flash"),
            ("thinking", "glm-z1-airx"),
        ]);

        // === 能力配置 ===
        inner.extend_capabilities(vec![
            // 常规模型
            ("glm-5", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING),
            ("glm-4", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING),
            ("glm-4-flash", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING),
            ("glm-4-plus", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING),
            ("glm-4-air", Capability::CHAT | Capability::VISION | Capability::STREAMING),
            ("glm-4-long", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING),
            // 思考模型
            ("glm-z1-airx", Capability::CHAT | Capability::STREAMING | Capability::THINKING),
            ("glm-z1-flash", Capability::CHAT | Capability::STREAMING | Capability::THINKING),
        ]);

        // === 上下文长度 ===
        inner.extend_context_lengths(vec![
            // 常规模型（128K）
            ("glm-5", 128_000),
            ("glm-4", 128_000),
            ("glm-4-flash", 128_000),
            ("glm-4-plus", 128_000),
            ("glm-4-air", 128_000),
            // 长上下文模型（1M）
            ("glm-4-long", 1_000_000),
            // 思考模型（128K）
            ("glm-z1-airx", 128_000),
            ("glm-z1-flash", 128_000),
        ]);

        Self { inner }
    }
}

impl Default for ZhipuModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for ZhipuModelResolver {
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
