use super::default::DefaultModelResolver;
use super::resolver::{ModelResolver, Capability};

/// Z.AI（智谱 GLM 海外版）模型解析器
///
/// Z.AI 是智谱 AI 的海外服务，提供 GLM 系列模型。
///
/// ## 支持的模型
///
/// | 模型 | 能力 | 上下文 |
/// |------|------|--------|
/// | glm-5 | Chat, Tools, Streaming | 128K |
/// | glm-5-flash | Chat, Tools, Streaming | 128K |
/// | glm-4 | Chat, Vision, Tools, Streaming | 128K |
/// | glm-4-flash | Chat, Tools, Streaming | 128K |
/// | glm-4-plus | Chat, Vision, Tools, Streaming | 128K |
/// | glm-4v | Chat, Vision, Streaming | 128K |
///
/// ## 便捷别名
///
/// | 别名 | 解析为 |
/// |------|--------|
/// | `glm` | `glm-5` |
/// | `glm5` | `glm-5` |
/// | `glm4` | `glm-4` |
/// | `flash` | `glm-4-flash` |
/// | `vision` | `glm-4v` |
///
/// ## 示例
///
/// ```
/// let resolver = ZaiModelResolver::new();
///
/// // 使用别名
/// assert_eq!(resolver.resolve("glm"), "glm-5");
/// assert_eq!(resolver.resolve("flash"), "glm-4-flash");
///
/// // 检查能力
/// assert!(resolver.has_capability("glm-4", Capability::VISION));
/// assert!(resolver.has_capability("glm-5", Capability::TOOLS));
/// ```
pub struct ZaiModelResolver {
    inner: DefaultModelResolver,
}

impl ZaiModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // === 模型别名 ===
        inner.extend_aliases(vec![
            // GLM-5 系列
            ("glm", "glm-5"),
            ("glm5", "glm-5"),
            ("glm-5", "glm-5"),
            ("glm5-flash", "glm-5-flash"),
            // GLM-4 系列
            ("glm4", "glm-4"),
            ("glm-4", "glm-4"),
            ("glm4-plus", "glm-4-plus"),
            ("glm-4-plus", "glm-4-plus"),
            ("flash", "glm-4-flash"),
            ("glm-flash", "glm-4-flash"),
            ("glm4-flash", "glm-4-flash"),
            ("glm-4-flash", "glm-4-flash"),
            // 视觉模型
            ("vision", "glm-4v"),
            ("glm-vision", "glm-4v"),
            ("glm4v", "glm-4v"),
            ("glm-4v", "glm-4v"),
        ]);

        // === 能力配置 ===
        // GLM-5 系列
        inner.extend_capabilities(vec![
            ("glm-5", Capability::CHAT | Capability::TOOLS | Capability::STREAMING),
            ("glm-5-flash", Capability::CHAT | Capability::TOOLS | Capability::STREAMING),
        ]);

        // GLM-4 系列
        inner.extend_capabilities(vec![
            ("glm-4", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING),
            ("glm-4-flash", Capability::CHAT | Capability::TOOLS | Capability::STREAMING),
            ("glm-4-plus", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING),
        ]);

        // 视觉模型
        inner.extend_capabilities(vec![
            ("glm-4v", Capability::CHAT | Capability::VISION | Capability::STREAMING),
        ]);

        // === 上下文长度 ===
        inner.extend_context_lengths(vec![
            ("glm-5", 128_000),
            ("glm-5-flash", 128_000),
            ("glm-4", 128_000),
            ("glm-4-flash", 128_000),
            ("glm-4-plus", 128_000),
            ("glm-4v", 128_000),
        ]);

        Self { inner }
    }
}

impl Default for ZaiModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for ZaiModelResolver {
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
