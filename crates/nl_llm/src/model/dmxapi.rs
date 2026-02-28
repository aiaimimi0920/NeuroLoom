use crate::model::{Capability, DefaultModelResolver, ModelResolver};

/// DMXAPI 模型解析器
///
/// ## 别名
///
/// | 别名 | 解析为 |
/// |------|--------|
/// | `dmxapi` / `sonnet` | `claude-sonnet-4-5-20250929` |
/// | `opus` | `claude-opus-4-6` |
/// | `4o` | `gpt-4o` |
/// | `4o-mini` | `gpt-4o-mini` |
/// | `4.1` | `gpt-4.1` |
/// | `4.1-mini` | `gpt-4.1-mini` |
///
/// ## 能力
///
/// - `CHAT`: 全部模型
/// - `TOOLS`: 全部模型
/// - `STREAMING`: 全部模型
/// - `VISION`: GPT-4o/Claude 系列
/// - `THINKING`: Claude 系列
pub struct DmxApiModelResolver {
    inner: DefaultModelResolver,
}

impl DmxApiModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        inner.extend_aliases(vec![
            ("dmxapi", "claude-sonnet-4-5-20250929"),
            ("sonnet", "claude-sonnet-4-5-20250929"),
            ("opus", "claude-opus-4-6"),
            ("4o", "gpt-4o"),
            ("4o-mini", "gpt-4o-mini"),
            ("4.1", "gpt-4.1"),
            ("4.1-mini", "gpt-4.1-mini"),
        ]);

        let claude_caps = Capability::CHAT
            | Capability::TOOLS
            | Capability::STREAMING
            | Capability::VISION
            | Capability::THINKING;
        let gpt_caps =
            Capability::CHAT | Capability::TOOLS | Capability::STREAMING | Capability::VISION;

        inner.extend_capabilities(vec![
            ("claude-sonnet-4-5-20250929", claude_caps),
            ("claude-opus-4-6", claude_caps),
            ("gpt-4o", gpt_caps),
            ("gpt-4o-mini", gpt_caps),
            ("gpt-4.1", gpt_caps),
            ("gpt-4.1-mini", gpt_caps),
        ]);

        inner.extend_context_lengths(vec![
            ("claude-sonnet-4-5-20250929", 200_000),
            ("claude-opus-4-6", 200_000),
            ("gpt-4o", 128_000),
            ("gpt-4o-mini", 128_000),
            ("gpt-4.1", 1_000_000),
            ("gpt-4.1-mini", 1_000_000),
        ]);

        Self { inner }
    }
}

impl Default for DmxApiModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for DmxApiModelResolver {
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
