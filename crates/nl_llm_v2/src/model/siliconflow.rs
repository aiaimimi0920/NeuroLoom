use crate::model::{Capability, DefaultModelResolver, ModelResolver};

/// SiliconFlow (硅基流动) 模型解析器
///
/// ## 模型命名规则
///
/// - `Pro/组织/模型` — 高性能推理层
/// - `组织/模型` — 标准推理层
/// - `Free/组织/模型` — 免费推理层
///
/// ## 别名
///
/// | 别名 | 解析为 |
/// |------|--------|
/// | `siliconflow` / `kimi-k2.5` | `Pro/moonshotai/Kimi-K2.5` |
/// | `deepseek-r1` / `r1` | `Pro/deepseek-ai/DeepSeek-R1` |
/// | `deepseek-v3` / `v3` | `Pro/deepseek-ai/DeepSeek-V3` |
/// | `qwen3-8b` | `Qwen/Qwen3-8B` |
/// | `qwen3-32b` | `Qwen/Qwen3-32B` |
/// | `qwen3-vl` | `Qwen/Qwen3-VL-72B-Instruct` |
/// | `qwen3-omni` | `Qwen/Qwen3-Omni-30B-A3B-Instruct` |
/// | `qwen2.5-vl` | `Qwen/Qwen2.5-VL-72B-Instruct` |
/// | `deepseek-vl2` | `deepseek-ai/deepseek-vl2` |
///
/// ## 能力
///
/// - `CHAT`: 全部模型
/// - `TOOLS`: 全部模型
/// - `STREAMING`: 全部模型
/// - `THINKING`: DeepSeek R1、Qwen3 系列
/// - `VISION`: VL 系列、DeepSeek VL2
pub struct SiliconFlowModelResolver {
    inner: DefaultModelResolver,
}

impl SiliconFlowModelResolver {
    pub fn new() -> Self {
        let mut inner = DefaultModelResolver::new();

        // === 模型别名 ===
        inner.extend_aliases(vec![
            // Pro 推理层
            ("siliconflow", "Pro/moonshotai/Kimi-K2.5"),
            ("kimi-k2.5", "Pro/moonshotai/Kimi-K2.5"),
            ("kimi", "Pro/moonshotai/Kimi-K2.5"),
            ("deepseek-r1", "Pro/deepseek-ai/DeepSeek-R1"),
            ("r1", "Pro/deepseek-ai/DeepSeek-R1"),
            ("deepseek-v3", "Pro/deepseek-ai/DeepSeek-V3"),
            ("v3", "Pro/deepseek-ai/DeepSeek-V3"),
            // 标准推理层
            ("qwen3-8b", "Qwen/Qwen3-8B"),
            ("qwen3-32b", "Qwen/Qwen3-32B"),
            // 多模态模型
            ("qwen3-vl", "Qwen/Qwen3-VL-72B-Instruct"),
            ("qwen3-omni", "Qwen/Qwen3-Omni-30B-A3B-Instruct"),
            ("qwen2.5-vl", "Qwen/Qwen2.5-VL-72B-Instruct"),
            ("deepseek-vl2", "deepseek-ai/deepseek-vl2"),
        ]);

        // === 能力配置 ===
        let standard_caps = Capability::CHAT | Capability::TOOLS | Capability::STREAMING;
        let thinking_caps = standard_caps | Capability::THINKING;
        let vision_caps = standard_caps | Capability::VISION;
        let vision_thinking_caps = standard_caps | Capability::VISION | Capability::THINKING;

        inner.extend_capabilities(vec![
            // === Pro 推理层 ===
            ("Pro/moonshotai/Kimi-K2.5", standard_caps),
            ("Pro/deepseek-ai/DeepSeek-R1", thinking_caps),
            ("Pro/deepseek-ai/DeepSeek-V3", standard_caps),
            // === 标准推理层 ===
            ("Qwen/Qwen3-8B", thinking_caps),
            ("Qwen/Qwen3-32B", thinking_caps),
            ("deepseek-ai/DeepSeek-V3", standard_caps),
            ("deepseek-ai/DeepSeek-R1", thinking_caps),
            // === 多模态模型 ===
            ("Qwen/Qwen3-VL-72B-Instruct", vision_thinking_caps),
            ("Qwen/Qwen3-Omni-30B-A3B-Instruct", vision_thinking_caps),
            ("Qwen/Qwen2.5-VL-72B-Instruct", vision_caps),
            ("deepseek-ai/deepseek-vl2", vision_caps),
        ]);

        // === 上下文长度 ===
        inner.extend_context_lengths(vec![
            // Pro 推理层
            ("Pro/moonshotai/Kimi-K2.5", 128_000),
            ("Pro/deepseek-ai/DeepSeek-R1", 64_000),
            ("Pro/deepseek-ai/DeepSeek-V3", 64_000),
            // 标准推理层
            ("Qwen/Qwen3-8B", 128_000),
            ("Qwen/Qwen3-32B", 128_000),
            ("deepseek-ai/DeepSeek-V3", 64_000),
            ("deepseek-ai/DeepSeek-R1", 64_000),
            // 多模态模型
            ("Qwen/Qwen3-VL-72B-Instruct", 128_000),
            ("Qwen/Qwen3-Omni-30B-A3B-Instruct", 128_000),
            ("Qwen/Qwen2.5-VL-72B-Instruct", 128_000),
            ("deepseek-ai/deepseek-vl2", 64_000),
        ]);

        Self { inner }
    }
}

impl Default for SiliconFlowModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for SiliconFlowModelResolver {
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
