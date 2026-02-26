use super::resolver::{Capability, ModelResolver};
use std::collections::HashMap;

/// 默认模型解析器
///
/// 作为默认实现，不包含任何平台特定的��置。
/// 各平台应使用专用的 ModelResolver：
/// - OpenAI: OpenAiModelResolver
/// - Claude: AnthropicModelResolver
/// - Gemini: GeminiModelResolver
/// - Qwen: QwenModelResolver
/// - Kimi: KimiModelResolver
/// - Codex: CodexModelResolver
pub struct DefaultModelResolver {
    /// 模型别名映射
    aliases: HashMap<String, String>,
    /// 模型能力表
    capabilities: HashMap<String, Capability>,
    /// 模型上下文长度表
    context_lengths: HashMap<String, usize>,
    /// 模型基础智能与模态表
    intelligence_profiles: HashMap<String, (f32, crate::model::resolver::Modality)>,
}

impl DefaultModelResolver {
    pub fn new() -> Self {
        Self {
            aliases: HashMap::new(),
            capabilities: HashMap::new(),
            context_lengths: HashMap::new(),
            intelligence_profiles: HashMap::new(),
        }
    }

    /// 设置或覆盖模型别名
    pub fn set_alias(&mut self, alias: impl Into<String>, model: impl Into<String>) {
        self.aliases.insert(alias.into(), model.into());
    }

    /// 设置模型能力
    pub fn set_capability(&mut self, model: impl Into<String>, cap: Capability) {
        self.capabilities.insert(model.into(), cap);
    }

    /// 设置模型上下文长度
    pub fn set_context_length(&mut self, model: impl Into<String>, length: usize) {
        self.context_lengths.insert(model.into(), length);
    }

    /// 设置模型智能等级与模态
    pub fn set_intelligence_profile(
        &mut self,
        model: impl Into<String>,
        score: f32,
        modality: crate::model::resolver::Modality,
    ) {
        self.intelligence_profiles
            .insert(model.into(), (score, modality));
    }

    /// 批量设置别名
    pub fn extend_aliases(&mut self, aliases: Vec<(impl Into<String>, impl Into<String>)>) {
        for (alias, model) in aliases {
            self.aliases.insert(alias.into(), model.into());
        }
    }

    /// 批量设置能力
    pub fn extend_capabilities(&mut self, caps: Vec<(impl Into<String>, Capability)>) {
        for (model, cap) in caps {
            self.capabilities.insert(model.into(), cap);
        }
    }

    /// 批量设置上下文长度
    pub fn extend_context_lengths(&mut self, lengths: Vec<(impl Into<String>, usize)>) {
        for (model, length) in lengths {
            self.context_lengths.insert(model.into(), length);
        }
    }

    /// 批量设置智能剖析
    pub fn extend_intelligence_profiles(
        &mut self,
        profiles: Vec<(impl Into<String>, f32, crate::model::resolver::Modality)>,
    ) {
        for (model, score, modality) in profiles {
            self.intelligence_profiles
                .insert(model.into(), (score, modality));
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
        self.aliases
            .get(model)
            .cloned()
            .unwrap_or_else(|| model.to_string())
    }

    fn has_capability(&self, model: &str, cap: Capability) -> bool {
        let resolved = self.resolve(model);
        self.capabilities
            .get(&resolved)
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

    fn intelligence_and_modality(
        &self,
        model: &str,
    ) -> Option<(f32, crate::model::resolver::Modality)> {
        let resolved = self.resolve(model);
        self.intelligence_profiles.get(&resolved).cloned()
    }
}
