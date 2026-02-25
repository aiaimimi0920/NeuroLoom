use crate::model::resolver::{ModelResolver, Capability};

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
        // Dify 默认应用都支持标准的聊天和流式返回
        cap.contains(Capability::CHAT) || cap.contains(Capability::STREAMING)
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
}
