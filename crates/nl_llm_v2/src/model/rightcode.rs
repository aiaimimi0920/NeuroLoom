use crate::model::{Capability, DefaultModelResolver, ModelResolver};

/// RightCode 模型解析器
///
/// ## 别名
///
/// | 别名 | 解析为 |
/// |------|--------|
/// | `rightcode` / `codex-mini` | `gpt-5.1-codex-mini` |
/// | `codex` | `gpt-5.1-codex` |
/// | `codex-max` | `gpt-5.1-codex-max` |
/// | `5` | `gpt-5` |
/// | `5.1` | `gpt-5.1` |
/// | `5.2` | `gpt-5.2` |
/// | `5.3` | `gpt-5.3-codex` |
///
/// ## 能力
///
/// - `CHAT`: 全部模型
/// - `TOOLS`: 全部模型
/// - `STREAMING`: 全部模型
/// - `VISION`: 全部模型
/// - `THINKING`: 全部模型
///
/// ## 上下文长度
///
/// 所有模型统一 200K context。
pub struct RightCodeModelResolver {
    inner: DefaultModelResolver,
}

impl RightCodeModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        inner.extend_aliases(vec![
            ("rightcode", "gpt-5.1-codex-mini"),
            ("codex", "gpt-5.1-codex"),
            ("codex-mini", "gpt-5.1-codex-mini"),
            ("codex-max", "gpt-5.1-codex-max"),
            ("5", "gpt-5"),
            ("5.1", "gpt-5.1"),
            ("5.2", "gpt-5.2"),
            ("5.3", "gpt-5.3-codex"),
        ]);

        let gpt_caps = Capability::CHAT | Capability::TOOLS | Capability::STREAMING | Capability::VISION | Capability::THINKING;

        // 所有模型统一 capabilities
        let all_models = [
            "gpt-5", "gpt-5-codex", "gpt-5-codex-mini",
            "gpt-5.1", "gpt-5.1-codex", "gpt-5.1-codex-max", "gpt-5.1-codex-mini",
            "gpt-5.2", "gpt-5.2-codex", "gpt-5.2-high", "gpt-5.2-medium", "gpt-5.2-low", "gpt-5.2-xhigh",
            "gpt-5.3-codex", "gpt-5.3-codex-high", "gpt-5.3-codex-medium", "gpt-5.3-codex-low", "gpt-5.3-codex-xhigh",
        ];
        inner.extend_capabilities(
            all_models.iter().map(|m| (*m, gpt_caps)).collect()
        );
        inner.extend_context_lengths(
            all_models.iter().map(|m| (*m, 200_000usize)).collect()
        );

        Self { inner }
    }
}

impl Default for RightCodeModelResolver {
    fn default() -> Self { Self::new() }
}

impl ModelResolver for RightCodeModelResolver {
    fn resolve(&self, model: &str) -> String { self.inner.resolve(model) }
    fn has_capability(&self, model: &str, cap: Capability) -> bool { self.inner.has_capability(model, cap) }
    fn max_context(&self, model: &str) -> usize { self.inner.max_context(model) }
    fn context_window_hint(&self, model: &str) -> (usize, usize) { self.inner.context_window_hint(model) }
    fn intelligence_and_modality(&self, _model: &str) -> Option<(f32, crate::model::resolver::Modality)> { None }

}
