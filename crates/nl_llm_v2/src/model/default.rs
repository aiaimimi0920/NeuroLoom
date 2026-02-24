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
        // Claude 别名 (Official API)
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
        // Claude (Official API)
        capabilities.insert("claude-sonnet-4-20250514".into(), Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING);
        capabilities.insert("claude-opus-4-20250514".into(), Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING);
        // Gemini
        capabilities.insert("gemini-2.5-flash".into(), Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING | Capability::THINKING);
        capabilities.insert("gemini-2.5-flash-lite".into(), Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING | Capability::THINKING);
        capabilities.insert("gemini-2.5-pro".into(), Capability::CHAT | Capability::VISION | Capability::TOOLS | Capability::STREAMING | Capability::THINKING | Capability::CODE_EXECUTION);

        let mut context_lengths = HashMap::new();
        context_lengths.insert("gpt-4o".into(), 128_000);
        context_lengths.insert("gpt-4-turbo".into(), 128_000);
        context_lengths.insert("claude-sonnet-4-20250514".into(), 200_000);
        context_lengths.insert("claude-opus-4-20250514".into(), 200_000);
        // Gemini
        context_lengths.insert("gemini-2.5-flash".into(), 1_000_000);
        context_lengths.insert("gemini-2.5-flash-lite".into(), 1_000_000);
        context_lengths.insert("gemini-2.5-pro".into(), 1_000_000);

        Self { aliases, capabilities, context_lengths }
    }

    /// [新增] 设置或覆盖模型别名
    /// 原因：允许预设级 ModelResolver 扩展或覆盖默认别名
    pub fn set_alias(&mut self, alias: impl Into<String>, model: impl Into<String>) {
        self.aliases.insert(alias.into(), model.into());
    }

    /// [新增] 设置模型能力
    pub fn set_capability(&mut self, model: impl Into<String>, cap: Capability) {
        self.capabilities.insert(model.into(), cap);
    }

    /// [新增] 设置模型上下文长度
    pub fn set_context_length(&mut self, model: impl Into<String>, length: usize) {
        self.context_lengths.insert(model.into(), length);
    }

    /// [新增] 批量设置别名
    pub fn extend_aliases(&mut self, aliases: Vec<(impl Into<String>, impl Into<String>)>) {
        for (alias, model) in aliases {
            self.aliases.insert(alias.into(), model.into());
        }
    }

    /// [新增] 批量设置能力
    pub fn extend_capabilities(&mut self, caps: Vec<(impl Into<String>, Capability)>) {
        for (model, cap) in caps {
            self.capabilities.insert(model.into(), cap);
        }
    }

    /// [新增] 批量设置上下文长度
    pub fn extend_context_lengths(&mut self, lengths: Vec<(impl Into<String>, usize)>) {
        for (model, length) in lengths {
            self.context_lengths.insert(model.into(), length);
        }
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
