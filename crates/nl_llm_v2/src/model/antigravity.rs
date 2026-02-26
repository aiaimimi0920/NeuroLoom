use super::default::DefaultModelResolver;
use super::resolver::{Capability, ModelResolver};

/// Antigravity / CloudCode PA 专属模型解析器
///
/// 基于默认解析器扩展，覆盖 Antigravity 平台特有的模型别名和能力：
/// - Claude 模型通过 CloudCode PA 翻译层支持
/// - Gemini 3.x 预览版使用内部名称
pub struct AntigravityModelResolver {
    inner: DefaultModelResolver,
}

impl AntigravityModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // === 别名覆盖 ===
        // Claude via Antigravity（覆盖默认的 Official API 别名）
        inner.set_alias("claude-opus", "claude-opus-4-6-thinking");
        inner.set_alias("claude-sonnet", "claude-sonnet-4-6");

        // Gemini 3.x 预览版 → CloudCode PA 内部名称
        inner.extend_aliases(vec![
            ("gemini-3-pro-preview", "gemini-3-pro-high"),
            ("gemini-3.1-pro-preview", "gemini-3.1-pro-high"),
            ("gemini-3-flash-preview", "gemini-3-flash"),
            ("gemini-3-pro-image-preview", "gemini-3-pro-image"),
        ]);

        // === 能力配置 ===
        // Claude via CloudCode PA
        inner.extend_capabilities(vec![
            (
                "claude-opus-4-6-thinking",
                Capability::CHAT
                    | Capability::VISION
                    | Capability::TOOLS
                    | Capability::STREAMING
                    | Capability::THINKING,
            ),
            (
                "claude-sonnet-4-6",
                Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING,
            ),
            (
                "claude-sonnet-4-6-thinking",
                Capability::CHAT
                    | Capability::VISION
                    | Capability::TOOLS
                    | Capability::STREAMING
                    | Capability::THINKING,
            ),
            (
                "claude-opus-4-5-thinking",
                Capability::CHAT
                    | Capability::VISION
                    | Capability::TOOLS
                    | Capability::STREAMING
                    | Capability::THINKING,
            ),
            (
                "claude-sonnet-4-5",
                Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING,
            ),
            (
                "claude-sonnet-4-5-thinking",
                Capability::CHAT
                    | Capability::VISION
                    | Capability::TOOLS
                    | Capability::STREAMING
                    | Capability::THINKING,
            ),
        ]);

        // Gemini 3.x (CloudCode PA 内部名)
        inner.extend_capabilities(vec![
            (
                "gemini-3-pro-high",
                Capability::CHAT
                    | Capability::VISION
                    | Capability::TOOLS
                    | Capability::STREAMING
                    | Capability::THINKING,
            ),
            (
                "gemini-3.1-pro-high",
                Capability::CHAT
                    | Capability::VISION
                    | Capability::TOOLS
                    | Capability::STREAMING
                    | Capability::THINKING,
            ),
            (
                "gemini-3.1-pro-low",
                Capability::CHAT
                    | Capability::VISION
                    | Capability::TOOLS
                    | Capability::STREAMING
                    | Capability::THINKING,
            ),
            (
                "gemini-3-flash",
                Capability::CHAT
                    | Capability::VISION
                    | Capability::TOOLS
                    | Capability::STREAMING
                    | Capability::THINKING,
            ),
            (
                "gemini-3-pro-image",
                Capability::CHAT
                    | Capability::VISION
                    | Capability::TOOLS
                    | Capability::STREAMING
                    | Capability::THINKING,
            ),
        ]);

        // === 上下文长度 ===
        // Claude via CloudCode PA (参照 GetAntigravityModelConfig MaxCompletionTokens)
        inner.extend_context_lengths(vec![
            ("claude-opus-4-6-thinking", 1_000_000),
            ("claude-sonnet-4-6", 200_000),
            ("claude-sonnet-4-6-thinking", 200_000),
            ("claude-opus-4-5-thinking", 200_000),
            ("claude-sonnet-4-5", 200_000),
            ("claude-sonnet-4-5-thinking", 200_000),
        ]);

        // Gemini 3.x
        inner.extend_context_lengths(vec![
            ("gemini-3-pro-high", 1_000_000),
            ("gemini-3.1-pro-high", 1_000_000),
            ("gemini-3.1-pro-low", 1_000_000),
            ("gemini-3-flash", 1_000_000),
            ("gemini-3-pro-image", 1_000_000),
        ]);

        Self { inner }
    }
}

impl Default for AntigravityModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for AntigravityModelResolver {
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
    fn intelligence_and_modality(
        &self,
        _model: &str,
    ) -> Option<(f32, crate::model::resolver::Modality)> {
        None
    }
}
