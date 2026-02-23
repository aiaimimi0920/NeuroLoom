use bitflags::bitflags;

bitflags! {
    /// 模型能力标志
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Capability: u32 {
        const CHAT = 0b0001;
        const VISION = 0b0010;
        const TOOLS = 0b0100;
        const STREAMING = 0b1000;
        const THINKING = 0b10000;
        const CODE_EXECUTION = 0b100000;
    }
}

/// 模型解析器
pub trait ModelResolver: Send + Sync {
    /// 解析模型别名到实际模型名
    /// 例如: "gpt4" → "gpt-4o", "claude" → "claude-sonnet-4-20250514"
    fn resolve(&self, model: &str) -> String;

    /// 检查模型是否支持指定能力
    fn has_capability(&self, model: &str, cap: Capability) -> bool;

    /// 获取模型的最大上下文长度
    fn max_context(&self, model: &str) -> usize;

    /// 获取模型的上下文窗口建议（输入/输出分配）
    fn context_window_hint(&self, model: &str) -> (usize, usize);
}
