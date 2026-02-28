use crate::model::{Capability, DefaultModelResolver, ModelResolver};

/// ModelScope (魔搭) 模型解析器
///
/// ## 支持的模型
///
/// ModelScope 使用 `组织/模型名` 格式，模型可用性取决于平台部署状态。
///
/// | 模型 | 上下文 | 说明 |
/// |------|--------|------|
/// | `Qwen/Qwen3-235B-A22B` | 128K | Qwen3 旗舰 MoE，支持思考模式 |
/// | `Qwen/Qwen3-32B` | 128K | Qwen3 32B Dense |
/// | `Qwen/Qwen3-8B` | 128K | Qwen3 8B 轻量 |
/// | `Qwen/Qwen2.5-Coder-32B-Instruct` | 128K | 编程专用 |
/// | `Qwen/QVQ-72B-Preview` | 128K | 视觉推理 |
///
/// ## 别名
///
/// | 别名 | 解析为 |
/// |------|--------|
/// | `modelscope` / `qwen3` | `Qwen/Qwen3-235B-A22B` |
/// | `qwen3-32b` | `Qwen/Qwen3-32B` |
/// | `qwen3-8b` | `Qwen/Qwen3-8B` |
/// | `coder` | `Qwen/Qwen2.5-Coder-32B-Instruct` |
/// | `qvq` | `Qwen/QVQ-72B-Preview` |
///
/// ## 能力
///
/// - `CHAT`: 支持对话
/// - `TOOLS`: 支持工具调用
/// - `STREAMING`: 支持流式输出
/// - `THINKING`: 支持思考模式 (Qwen3)
/// - `VISION`: 支持视觉理解 (QVQ)
pub struct ModelScopeModelResolver {
    inner: DefaultModelResolver,
}

impl ModelScopeModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // === 模型别名 ===
        inner.extend_aliases(vec![
            ("modelscope", "Qwen/Qwen3-235B-A22B"),
            ("qwen3", "Qwen/Qwen3-235B-A22B"),
            ("qwen3-32b", "Qwen/Qwen3-32B"),
            ("qwen3-8b", "Qwen/Qwen3-8B"),
            ("coder", "Qwen/Qwen2.5-Coder-32B-Instruct"),
            ("qvq", "Qwen/QVQ-72B-Preview"),
        ]);

        // === 能力配置 ===
        let thinking_caps =
            Capability::CHAT | Capability::TOOLS | Capability::STREAMING | Capability::THINKING;
        let standard_caps = Capability::CHAT | Capability::TOOLS | Capability::STREAMING;
        let vision_caps =
            Capability::CHAT | Capability::VISION | Capability::STREAMING | Capability::THINKING;

        inner.extend_capabilities(vec![
            ("Qwen/Qwen3-235B-A22B", thinking_caps),
            ("Qwen/Qwen3-32B", thinking_caps),
            ("Qwen/Qwen3-8B", thinking_caps),
            ("Qwen/Qwen2.5-Coder-32B-Instruct", standard_caps),
            ("Qwen/QVQ-72B-Preview", vision_caps),
        ]);

        // === 上下文长度 ===
        inner.extend_context_lengths(vec![
            ("Qwen/Qwen3-235B-A22B", 128_000),
            ("Qwen/Qwen3-32B", 128_000),
            ("Qwen/Qwen3-8B", 128_000),
            ("Qwen/Qwen2.5-Coder-32B-Instruct", 128_000),
            ("Qwen/QVQ-72B-Preview", 128_000),
        ]);

        Self { inner }
    }
}

impl Default for ModelScopeModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for ModelScopeModelResolver {
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
