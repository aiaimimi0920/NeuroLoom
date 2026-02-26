use super::resolver::{ModelResolver, Capability, Modality};

/// SubModel 模型解析器
///
/// SubModel 代理了一系列开源和闭源的高能模型。
pub struct SubModelModelResolver {}

impl SubModelModelResolver {
    pub fn new() -> Self {
        Self {}
    }
}

impl ModelResolver for SubModelModelResolver {
    fn resolve(&self, model: &str) -> String {
        // 由于 SubModel 定义了较长的具体模型名称
        // 我们提供一些便捷短名映射
        match model.to_lowercase().as_str() {
            "hermes" | "hermes-4-405b-fp8" => "NousResearch/Hermes-4-405B-FP8".to_string(),
            "qwen-thinking" | "qwen-3-thinking" => "Qwen/Qwen3-235B-A22B-Thinking-2507".to_string(),
            "qwen-coder" | "qwen3-coder" => "Qwen/Qwen3-Coder-480B-A35B-Instruct-FP8".to_string(),
            "qwen" | "qwen3" => "Qwen/Qwen3-235B-A22B-Instruct-2507".to_string(),
            "glm" | "glm-4.5" => "zai-org/GLM-4.5-FP8".to_string(),
            "gpt-oss" | "gpt-oss-120b" => "openai/gpt-oss-120b".to_string(),
            "r1" | "deepseek-r1" => "deepseek-ai/DeepSeek-R1".to_string(),
            "v3" | "deepseek-v3" => "deepseek-ai/DeepSeek-V3.1".to_string(),
            
            _ => model.to_string(), // 否则直接透传给远端
        }
    }

    fn has_capability(&self, _model: &str, capability: Capability) -> bool {
        // 核心文字交流、流式等能力全线支持
        matches!(
            capability,
            Capability::CHAT | Capability::STREAMING | Capability::TOOLS
        )
    }

    fn context_window_hint(&self, model: &str) -> (usize, usize) {
        let name = model.to_lowercase();
        if name.contains("qwen3") {
            (128000, 4096)
        } else if name.contains("deepseek-v3") || name.contains("deepseek-r1") || name.contains("glm") {
            (64000, 8192)
        } else {
            (128000, 4096)
        }
    }

    fn max_context(&self, model: &str) -> usize {
        let (max, _) = self.context_window_hint(model);
        max
    }

    fn intelligence_and_modality(&self, model: &str) -> Option<(f32, Modality)> {
        let name = model.to_lowercase();
        if name.contains("r1") || name.contains("v3") || name.contains("qwen3") || name.contains("glm") {
            Some((4.5, Modality::Text))
        } else {
            Some((4.0, Modality::Text))
        }
    }
}
