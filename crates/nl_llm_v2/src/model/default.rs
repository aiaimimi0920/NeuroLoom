use super::resolver::{ModelResolver, Capability};
use std::collections::HashMap;

/// 默认模型解析器
pub struct DefaultModelResolver {
    /// 模型别名映射
    aliases: HashMap<String, String>,
    /// 模型能力表
    capabilities: HashMap<String, Capability>,
    /// 模型上下文长度表
    context_lengths: HashMap<String, usize>,
}

impl DefaultModelResolver {
    pub fn new() -> Self {
        let mut aliases = HashMap::new();
        // OpenAI 别名
        aliases.insert("gpt4".into(), "gpt-4o".into());
        aliases.insert("gpt4-turbo".into(), "gpt-4-turbo".into());
        aliases.insert("gpt3".into(), "gpt-3.5-turbo".into());
        // Claude 别名
        aliases.insert("claude".into(), "claude-sonnet-4-20250514".into());
        aliases.insert("claude-opus".into(), "claude-opus-4-20250514".into());
        aliases.insert("claude-sonnet".into(), "claude-sonnet-4-20250514".into());
        // Gemini 别名
        aliases.insert("gemini".into(), "gemini-2.5-flash".into());
        aliases.insert("gemini-pro".into(), "gemini-2.5-pro".into());
        aliases.insert("gemini-flash".into(), "gemini-2.5-flash".into());

        let mut capabilities = HashMap::new();
        // OpenAI
        capabilities.insert("gpt-4o".into(), Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING);
        capabilities.insert("gpt-4-turbo".into(), Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING);
        // Claude
        capabilities.insert("claude-sonnet-4-20250514".into(), Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING);
        capabilities.insert("claude-opus-4-20250514".into(), Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING);
        // Gemini
        capabilities.insert("gemini-2.5-flash".into(), Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING | Capability::THINKING);
        capabilities.insert("gemini-2.5-pro".into(), Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING | Capability::THINKING | Capability::CODE_EXECUTION);

        let mut context_lengths = HashMap::new();
        context_lengths.insert("gpt-4o".into(), 128_000);
        context_lengths.insert("gpt-4-turbo".into(), 128_000);
        context_lengths.insert("claude-sonnet-4-20250514".into(), 200_000);
        context_lengths.insert("claude-opus-4-20250514".into(), 200_000);
        context_lengths.insert("gemini-2.5-flash".into(), 1_000_000);
        context_lengths.insert("gemini-2.5-pro".into(), 1_000_000);

        Self { aliases, capabilities, context_lengths }
    }
}

impl Default for DefaultModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for DefaultModelResolver {
    fn resolve(&self, model: &str) -> String {
        self.aliases.get(model).cloned().unwrap_or_else(|| model.to_string())
    }

    fn has_capability(&self, model: &str, cap: Capability) -> bool {
        let resolved = self.resolve(model);
        self.capabilities.get(&resolved)
            .map(|c| c.contains(cap))
            .unwrap_or(false)
    }

    fn max_context(&self, model: &str) -> usize {
        let resolved = self.resolve(model);
        self.context_lengths.get(&resolved).copied().unwrap_or(4096)
    }

    fn context_window_hint(&self, model: &str) -> (usize, usize) {
        let max = self.max_context(model);
        // 默认保留 1/4 作为输出
        (max * 3 / 4, max / 4)
    }
}
