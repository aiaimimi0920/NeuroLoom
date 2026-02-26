use super::default::DefaultModelResolver;
use super::resolver::{ModelResolver, Capability};

/// Qwen (通义千问) 平台模型解析器
///
/// 阿里云百炼平台提供的 DashScope 兼容 API，支持全系列 Qwen 模型。
///
/// ## 支持的主流模型
///
/// | 模型 | 能力 | 上下文 | 说明 |
/// |------|------|--------|------|
/// | `qwen-max` | Chat, Vision, Tools, Streaming | 32K | 旗舰模型，全能冠军 |
/// | `qwen-plus` | Chat, Vision, Tools, Streaming | 128K | 均衡模型，性价比高 |
/// | `qwen-turbo` | Chat, Vision, Tools, Streaming | 128K | 极速模型，低延迟 |
/// | `qwen2.5-coder-32b-instruct` | Chat, Tools, Streaming | 32K | 顶级开源代码模型 |
/// | `qwen-vl-max` | Chat, Vision, Streaming | 32K | 多模态视觉旗舰 |
/// | `qwq-plus` | Chat, Streaming, Thinking | 128K | 推理思考模型 |
///
/// ## 常用别名
///
/// | 别名 | 解析为 | 说明 |
/// |------|--------|------|
/// | `qwen` / `plus` | `qwen-plus` | 默认均衡模型 |
/// | `max` | `qwen-max` | 旗舰大核 |
/// | `turbo` | `qwen-turbo` | 轻量提速 |
/// | `coder` / `code` | `qwen2.5-coder-32b-instruct` | 编码专家 |
/// | `vl` / `vision` | `qwen-vl-max` | 视觉多模态 |
/// | `qwq` | `qwq-plus` | 推理思考 |
pub struct QwenModelResolver {
    inner: DefaultModelResolver,
}

impl QwenModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // ========== 模型别名 ==========
        inner.extend_aliases(vec![
            // 旗舰通用系列
            ("qwen", "qwen-plus"),
            ("plus", "qwen-plus"),
            ("max", "qwen-max"),
            ("turbo", "qwen-turbo"),
            ("latest", "qwen-max-latest"),

            // 代码模型专用
            ("coder", "qwen2.5-coder-32b-instruct"),
            ("code", "qwen2.5-coder-32b-instruct"),
            ("coder-32b", "qwen2.5-coder-32b-instruct"),
            ("coder-14b", "qwen2.5-coder-14b-instruct"),
            ("coder-7b", "qwen2.5-coder-7b-instruct"),

            // 视觉模型
            ("vl", "qwen-vl-max"),
            ("vision", "qwen-vl-max"),
            ("vl-max", "qwen-vl-max"),
            ("vl-plus", "qwen-vl-plus"),

            // 推理思考模型 (QwQ)
            ("qwq", "qwq-plus"),
            ("thinking", "qwq-plus"),
            ("qwq-32b", "qwq-32b-preview"),
        ]);

        // ========== 能力配置 ==========
        // 目前大部分 Qwen 模型均支持 Tools (Function Calling)
        // 主力模型（max, plus, turbo）已支持 Vision 多模态
        inner.extend_capabilities(vec![
            // 主力商业模型
            ("qwen-max", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING),
            ("qwen-plus", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING),
            ("qwen-turbo", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING),
            ("qwen-max-latest", Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING | Capability::THINKING),

            // 专属代码模型
            ("qwen2.5-coder-32b-instruct", Capability::CHAT | Capability::TOOLS | Capability::STREAMING),
            ("qwen2.5-coder-14b-instruct", Capability::CHAT | Capability::TOOLS | Capability::STREAMING),
            ("qwen2.5-coder-7b-instruct", Capability::CHAT | Capability::TOOLS | Capability::STREAMING),

            // 基础开源模型
            ("qwen2.5-72b-instruct", Capability::CHAT | Capability::TOOLS | Capability::STREAMING),
            ("qwen2.5-14b-instruct", Capability::CHAT | Capability::TOOLS | Capability::STREAMING),
            ("qwen2.5-7b-instruct", Capability::CHAT | Capability::TOOLS | Capability::STREAMING),

            // 视觉模型 (支持 Vision，通常也支持基本对话)
            ("qwen-vl-max", Capability::CHAT | Capability::VISION | Capability::STREAMING),
            ("qwen-vl-plus", Capability::CHAT | Capability::VISION | Capability::STREAMING),

            // 推理思考模型 (QwQ)
            ("qwq-plus", Capability::CHAT | Capability::STREAMING | Capability::THINKING),
            ("qwq-32b-preview", Capability::CHAT | Capability::STREAMING | Capability::THINKING),
        ]);

        // ========== 上下文长度 ==========
        inner.extend_context_lengths(vec![
            ("qwen-max", 32_768),            // Max 当前通常是 32k
            ("qwen-max-latest", 32_768),
            ("qwen-plus", 131_072),          // Plus 支持近 130k
            ("qwen-turbo", 131_072),
            ("qwen2.5-coder-32b-instruct", 32_768), // Coder 32k
            ("qwen2.5-coder-14b-instruct", 32_768),
            ("qwen2.5-coder-7b-instruct", 32_768),
            ("qwen2.5-72b-instruct", 131_072),
            ("qwen2.5-14b-instruct", 131_072),
            ("qwen2.5-7b-instruct", 131_072),
            ("qwen-vl-max", 32_768),
            ("qwen-vl-plus", 32_768),
            // QwQ 推理模型
            ("qwq-plus", 131_072),
            ("qwq-32b-preview", 131_072),
        ]);

        Self { inner }
    }
}

impl Default for QwenModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for QwenModelResolver {
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
