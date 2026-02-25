use crate::model::resolver::{ModelResolver, Capability};

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

    fn has_capability(&self, _model: &str, cap: Capability) -> bool {
        // 赋予标准功能：虽然 Jina 的核心竞争在 Embed，但其 /chat/completions 兼容标准协议
        cap.contains(Capability::CHAT) || cap.contains(Capability::STREAMING)
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
