use crate::model::resolver::{Capability, ModelResolver};

/// Jina 模型解析器
/// Jina 包含嵌入模型 (jina-embeddings-v3) 和聊天模型等。
#[derive(Debug, Clone, Default)]
pub struct JinaModelResolver {}

impl JinaModelResolver {
    pub fn new() -> Self {
        Self {}
    }
}

impl ModelResolver for JinaModelResolver {
    fn resolve(&self, model: &str) -> String {
        // Jina 模型名称保持原样传递给 API
        model.to_string()
    }

    fn has_capability(&self, model: &str, cap: Capability) -> bool {
        // Jina 平台同时提供 chat/completions 和 embeddings。
        // 嵌入模型不应被误判为对话模型，避免上层调度时出现能力误判。
        let resolved = self.resolve(model);
        let model_caps = if resolved.contains("embeddings") {
            Capability::empty()
        } else {
            Capability::CHAT | Capability::STREAMING
        };

        model_caps.contains(cap)
    }

    fn max_context(&self, _model: &str) -> usize {
        128_000 // 赋予一个默认大上下文环境避免报错，具体视 Jina 模型而定
    }

    fn context_window_hint(&self, model: &str) -> (usize, usize) {
        let max = self.max_context(model);
        let input_limit = max * 3 / 4;
        let output_limit = max - input_limit;
        (input_limit, output_limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_models_support_chat_and_stream() {
        let resolver = JinaModelResolver::new();

        assert!(resolver.has_capability("jina-chat", Capability::CHAT));
        assert!(resolver.has_capability("jina-chat", Capability::STREAMING));
    }

    #[test]
    fn embeddings_models_are_not_marked_as_chat_capable() {
        let resolver = JinaModelResolver::new();

        assert!(!resolver.has_capability("jina-embeddings-v3", Capability::CHAT));
        assert!(!resolver.has_capability("jina-embeddings-v3", Capability::STREAMING));
    }

    #[test]
    fn combined_capability_check_requires_all_flags() {
        let resolver = JinaModelResolver::new();

        assert!(!resolver.has_capability("jina-chat", Capability::CHAT | Capability::TOOLS));
    }
}
