use crate::model::{Capability, ModelResolver};

/// AI Proxy 模型解析器
/// 
/// AI Proxy 作为一个兼容了 OpenAI 的聚合代理平台，
/// 支持成百上千种模型（如 claude, openai, gemini 等），
/// 所以统一放行对应的 CHAT 和 STREAMING 能力，
/// 默认兜底使用 gpt-4o。
pub struct AiProxyModelResolver;

impl AiProxyModelResolver {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for AiProxyModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for AiProxyModelResolver {
    fn resolve(&self, model: &str) -> String {
        if model.is_empty() {
            "gpt-4o".to_string()
        } else {
            model.to_string()
        }
    }

    fn has_capability(&self, _model: &str, capability: Capability) -> bool {
        // AI Proxy 聚合各类模型，这里全部放行对讲和流式，以兼顾各种模型
        capability.contains(Capability::CHAT) || capability.contains(Capability::STREAMING)
    }

    fn max_context(&self, _model: &str) -> usize {
        // 动态池，一律给一个大的通用上下文边界
        128_000
    }

    fn context_window_hint(&self, model: &str) -> (usize, usize) {
        let max = self.max_context(model);
        (max * 3 / 4, max / 4)
    }
}
