use crate::model::{Capability, DefaultModelResolver, ModelResolver};

/// AiHubMix 聚合平台模型解析器
///
/// ## 别名
///
/// | 别名 | 解析为 |
/// |------|--------|
/// | `aihubmix` / `4o` | `gpt-4o-free` |
/// | `4.1` | `gpt-4.1-free` |
/// | `4.1-mini` | `gpt-4.1-mini-free` |
/// | `4.1-nano` | `gpt-4.1-nano-free` |
/// | `gemini` / `flash` | `gemini-2.0-flash-free` |
/// | `gemini3` | `gemini-3-flash-preview-free` |
/// | `glm` | `glm-4.7-flash-free` |
/// | `step` | `step-3.5-flash-free` |
/// | `sonnet` | `claude-sonnet-4-5-20250929` |
/// | `opus` | `claude-opus-4-6` |
///
/// ## 能力
///
/// - `CHAT`: 全部模型
/// - `TOOLS`: GPT/Claude/Gemini 系列
/// - `STREAMING`: 全部模型
/// - `THINKING`: Gemini 3/Claude 系列
/// - `VISION`: GPT-4o/Gemini/Claude 系列
pub struct AiHubMixModelResolver {
    inner: DefaultModelResolver,
}

impl AiHubMixModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // === 模型别名 ===
        inner.extend_aliases(vec![
            // 默认 → gpt-4o-free
            ("aihubmix", "gpt-4o-free"),
            ("4o", "gpt-4o-free"),
            // GPT 4.1 系列
            ("4.1", "gpt-4.1-free"),
            ("4.1-mini", "gpt-4.1-mini-free"),
            ("4.1-nano", "gpt-4.1-nano-free"),
            // Gemini 系列
            ("gemini", "gemini-2.0-flash-free"),
            ("flash", "gemini-2.0-flash-free"),
            ("gemini3", "gemini-3-flash-preview-free"),
            // GLM 系列
            ("glm", "glm-4.7-flash-free"),
            ("coding-glm", "coding-glm-5-free"),
            // 其他
            ("step", "step-3.5-flash-free"),
            ("minimax", "coding-minimax-m2-free"),
            // 付费
            ("sonnet", "claude-sonnet-4-5-20250929"),
            ("opus", "claude-opus-4-6"),
        ]);

        // === 能力配置 ===
        let gpt_caps = Capability::CHAT | Capability::TOOLS | Capability::STREAMING | Capability::VISION;
        let gemini_caps = Capability::CHAT | Capability::TOOLS | Capability::STREAMING | Capability::VISION;
        let gemini3_caps = gemini_caps | Capability::THINKING;
        let claude_caps = Capability::CHAT | Capability::TOOLS | Capability::STREAMING | Capability::VISION | Capability::THINKING;
        let standard_caps = Capability::CHAT | Capability::TOOLS | Capability::STREAMING;

        inner.extend_capabilities(vec![
            // GPT 系列
            ("gpt-4o-free", gpt_caps),
            ("gpt-4.1-free", gpt_caps),
            ("gpt-4.1-mini-free", gpt_caps),
            ("gpt-4.1-nano-free", gpt_caps),
            // Gemini 系列
            ("gemini-2.0-flash-free", gemini_caps),
            ("gemini-3-flash-preview-free", gemini3_caps),
            // GLM 系列
            ("glm-4.7-flash-free", standard_caps),
            ("coding-glm-5-free", standard_caps),
            ("coding-glm-4.7-free", standard_caps),
            ("coding-glm-4.6-free", standard_caps),
            // 其他
            ("step-3.5-flash-free", standard_caps),
            ("coding-minimax-m2-free", standard_caps),
            // 付费
            ("claude-sonnet-4-5-20250929", claude_caps),
            ("claude-opus-4-6", claude_caps),
        ]);

        // === 上下文长度 ===
        inner.extend_context_lengths(vec![
            ("gpt-4o-free", 1_000_000),
            ("gpt-4.1-free", 1_000_000),
            ("gpt-4.1-mini-free", 1_000_000),
            ("gpt-4.1-nano-free", 1_000_000),
            ("gemini-2.0-flash-free", 1_000_000),
            ("gemini-3-flash-preview-free", 1_000_000),
            ("glm-4.7-flash-free", 128_000),
            ("coding-glm-5-free", 128_000),
            ("coding-glm-4.7-free", 128_000),
            ("coding-glm-4.6-free", 128_000),
            ("step-3.5-flash-free", 256_000),
            ("coding-minimax-m2-free", 128_000),
            ("claude-sonnet-4-5-20250929", 200_000),
            ("claude-opus-4-6", 200_000),
        ]);

        Self { inner }
    }
}

impl Default for AiHubMixModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for AiHubMixModelResolver {
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
