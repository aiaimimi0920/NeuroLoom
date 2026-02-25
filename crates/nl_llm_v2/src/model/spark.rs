use crate::model::{Capability, DefaultModelResolver, ModelResolver};

/// 讯飞星火模型解析器
///
/// ## 支持的模型
///
/// | 模型 | 能力 | 上下文 | 说明 |
/// |------|------|--------|------|
/// | `4.0Ultra` | Chat, Tools, Streaming | 128K | 旗舰 |
/// | `max-32k` | Chat, Tools, Streaming | 32K | 长文本 |
/// | `generalv3.5` | Chat, Tools, Streaming | 128K | Spark Max |
/// | `pro-128k` | Chat, Streaming | 128K | 长上下文 |
/// | `generalv3` | Chat, Streaming | 8K | Spark Pro |
/// | `lite` | Chat, Streaming | 4K | 免费轻量 |
///
/// ## 常用别名
///
/// | 别名 | 解析为 | 说明 |
/// |------|--------|------|
/// | `spark` / `ultra` | `4.0Ultra` | 旗舰模型 |
/// | `max` | `generalv3.5` | 通用旗舰 |
/// | `max32k` | `max-32k` | 长文本 |
/// | `pro` | `generalv3` | 通用型 |
/// | `pro128k` | `pro-128k` | 长上下文 |
/// | `lite` | `lite` | 免费轻量 |
pub struct SparkModelResolver {
    inner: DefaultModelResolver,
}

impl SparkModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // ========== 模型别名 ==========
        inner.extend_aliases(vec![
            ("spark", "4.0Ultra"),
            ("ultra", "4.0Ultra"),
            ("4.0ultra", "4.0Ultra"),
            ("max", "generalv3.5"),
            ("spark-max", "generalv3.5"),
            ("v3.5", "generalv3.5"),
            ("max32k", "max-32k"),
            ("max-32k", "max-32k"),
            ("pro", "generalv3"),
            ("spark-pro", "generalv3"),
            ("v3", "generalv3"),
            ("pro128k", "pro-128k"),
            ("pro-128k", "pro-128k"),
            ("lite", "lite"),
            ("spark-lite", "lite"),
            // Spark X 系列
            ("x2", "spark-x"),
            ("spark-x", "spark-x"),
            ("sparkx", "spark-x"),
            ("x1", "spark-x"),
            ("x1.5", "spark-x"),
        ]);

        // ========== 能力配置 ==========
        let full_caps = Capability::CHAT | Capability::TOOLS | Capability::STREAMING;
        let basic_caps = Capability::CHAT | Capability::STREAMING;

        inner.extend_capabilities(vec![
            ("4.0Ultra", full_caps),
            ("max-32k", full_caps),
            ("generalv3.5", full_caps),
            ("pro-128k", basic_caps),
            ("generalv3", basic_caps),
            ("lite", basic_caps),
            // Spark X 系列
            ("spark-x", full_caps),
        ]);

        // ========== 上下文长度 ==========
        inner.extend_context_lengths(vec![
            ("4.0Ultra", 128_000),
            ("max-32k", 32_000),
            ("generalv3.5", 128_000),
            ("pro-128k", 128_000),
            ("generalv3", 8_000),
            ("lite", 4_000),
            // Spark X 系列
            ("spark-x", 128_000),
        ]);

        Self { inner }
    }
}

impl Default for SparkModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for SparkModelResolver {
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
