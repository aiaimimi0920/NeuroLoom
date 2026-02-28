use crate::model::{Capability, DefaultModelResolver, ModelResolver};

/// AWS Claude (Bedrock) 模型解析器
///
/// Bedrock 模型 ID 使用 `anthropic.claude-xxx` 格式（带版本号后缀）。
///
/// ## 支持的模型别名
///
/// | 别名 | 解析为 |
/// |------|--------|
/// | `aws` / `sonnet` / `claude` | Claude Sonnet 4.6 |
/// | `sonnet-4.6` | Claude Sonnet 4.6 |
/// | `sonnet-4.5` | Claude Sonnet 4.5 |
/// | `opus` | Claude Opus 4.6 |
/// | `opus-4.6` | Claude Opus 4.6 |
/// | `opus-4.5` | Claude Opus 4.5 |
/// | `3.5` | Claude 3.5 Sonnet v2 |
/// | `haiku` | Claude 3.5 Haiku |
pub struct AwsClaudeModelResolver {
    inner: DefaultModelResolver,
}

impl AwsClaudeModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // ============================================================
        //  模型别名
        // ============================================================
        inner.extend_aliases(vec![
            // Claude 4.6 系列
            ("aws", "anthropic.claude-sonnet-4-6-20250514-v1:0"),
            ("sonnet", "anthropic.claude-sonnet-4-6-20250514-v1:0"),
            ("claude", "anthropic.claude-sonnet-4-6-20250514-v1:0"),
            ("sonnet-4.6", "anthropic.claude-sonnet-4-6-20250514-v1:0"),
            ("opus", "anthropic.claude-opus-4-6-20250514-v1:0"),
            ("opus-4.6", "anthropic.claude-opus-4-6-20250514-v1:0"),
            // Claude 4.5 系列
            ("sonnet-4.5", "anthropic.claude-sonnet-4-5-20250929-v1:0"),
            ("opus-4.5", "anthropic.claude-opus-4-5-20250915-v1:0"),
            // Claude 3.5 系列
            ("3.5", "anthropic.claude-3-5-sonnet-20241022-v2:0"),
            ("haiku", "anthropic.claude-3-5-haiku-20241022-v1:0"),
            ("haiku-3", "anthropic.claude-3-haiku-20240307-v1:0"),
        ]);

        // ============================================================
        //  模型能力
        // ============================================================
        let claude_caps = Capability::CHAT
            | Capability::TOOLS
            | Capability::STREAMING
            | Capability::VISION
            | Capability::THINKING;

        let all_models = [
            // Claude 4.6 系列
            "anthropic.claude-sonnet-4-6-20250514-v1:0",
            "anthropic.claude-opus-4-6-20250514-v1:0",
            // Claude 4.5 系列
            "anthropic.claude-sonnet-4-5-20250929-v1:0",
            "anthropic.claude-opus-4-5-20250915-v1:0",
            // Claude 3.5 系列
            "anthropic.claude-3-5-sonnet-20241022-v2:0",
            "anthropic.claude-3-5-haiku-20241022-v1:0",
            "anthropic.claude-3-haiku-20240307-v1:0",
        ];

        inner.extend_capabilities(all_models.iter().map(|m| (*m, claude_caps)).collect());
        inner.extend_context_lengths(all_models.iter().map(|m| (*m, 200_000usize)).collect());

        Self { inner }
    }
}

impl Default for AwsClaudeModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for AwsClaudeModelResolver {
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
