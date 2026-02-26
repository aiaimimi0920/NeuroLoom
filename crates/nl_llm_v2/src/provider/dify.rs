use crate::model::resolver::{Capability, ModelResolver};

/// Dify 模型解析器
/// Dify App 隐藏了其底层大模型设定，因此我们默认所有输入均指向默认的 Chat 应用，并赋予对话和流式能力。
#[derive(Debug, Clone, Default)]
pub struct DifyModelResolver {}

impl DifyModelResolver {
    pub fn new() -> Self {
        Self {}
    }
}

impl ModelResolver for DifyModelResolver {
    fn resolve(&self, model: &str) -> String {
        // Dify 不需要显式的 model 名称，保持原样即可
        model.to_string()
    }

    fn has_capability(&self, _model: &str, cap: Capability) -> bool {
        // Dify 默认应用都支持标准的聊天和流式返回。
        // 注意：这里的 `cap` 可能是组合能力（例如 CHAT|TOOLS），因此必须用
        // `supported.contains(cap)` 语义来保证“请求的每一项能力都被支持”。
        let supported = Capability::CHAT | Capability::STREAMING;
        supported.contains(cap)
    }

    fn max_context(&self, _model: &str) -> usize {
        // 取决于后端的具体模型配置，默认给一个比较大的值防止截断
        128_000
    }

    fn context_window_hint(&self, model: &str) -> (usize, usize) {
        let max = self.max_context(model);
        // 按 3:1 比例分配输入输出
        let input_limit = max * 3 / 4;
        let output_limit = max - input_limit;
        (input_limit, output_limit)
    }

    fn intelligence_and_modality(
        &self,
        _model: &str,
    ) -> Option<(f32, crate::model::resolver::Modality)> {
        Some((3.5, crate::model::resolver::Modality::Text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supports_chat_and_streaming_only() {
        let resolver = DifyModelResolver::new();

        assert!(resolver.has_capability("dify", Capability::CHAT));
        assert!(resolver.has_capability("dify", Capability::STREAMING));

        assert!(!resolver.has_capability("dify", Capability::TOOLS));
        assert!(!resolver.has_capability("dify", Capability::VISION));
        assert!(!resolver.has_capability("dify", Capability::CHAT | Capability::TOOLS));
    }
}
