use crate::model::{Capability, ModelResolver};

/// 适用于“自定义渠道”的模型解析器
/// 
/// 自定义渠道不限制模型名，也不会进行预置模型校验。
/// 对于所有的模型，它都会直接透传，并盲目默认该模型支持 Chat 和 Streaming 功能。
pub struct CustomModelResolver;

impl CustomModelResolver {
    pub fn new() -> Self {
        Self
    }
}

impl ModelResolver for CustomModelResolver {
    fn resolve(&self, model: &str) -> String {
        // 自定义渠道不做别名映射，原样透传
        model.to_string()
    }

    fn has_capability(&self, _model: &str, cap: Capability) -> bool {
        // 默认统统支持常见的生成流传输能力，将校验推迟到服务端
        cap.contains(Capability::CHAT) || cap.contains(Capability::STREAMING)
    }

    fn max_context(&self, _model: &str) -> usize {
        // 泛用性地赋予一个最大的通用 Context Limit，依赖实际的自定义服务器配置起效
        128_000
    }

    fn context_window_hint(&self, model: &str) -> (usize, usize) {
        let max = self.max_context(model);
        (max * 3 / 4, max / 4)
    }
}
