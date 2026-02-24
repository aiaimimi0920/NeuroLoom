use std::collections::HashMap;
use once_cell::sync::Lazy;

use crate::client::ClientBuilder;

type BuilderFn = fn() -> ClientBuilder;

pub struct PresetRegistry {
    builders: HashMap<&'static str, BuilderFn>,
}

impl PresetRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            builders: HashMap::new(),
        };
        registry.register_defaults();
        registry
    }

    fn register_defaults(&mut self) {
        self.builders.insert("openai", super::openai::builder);
        self.builders.insert("claude", super::claude::builder);
        self.builders.insert("gemini", super::gemini::builder);
        self.builders.insert("vertex", super::vertex::builder);
        self.builders.insert("deepseek", super::deepseek::builder);
        self.builders.insert("moonshot", super::moonshot::builder);
        self.builders.insert("zhipu", super::zhipu::builder);
        self.builders.insert("iflow", super::iflow::builder);
        self.builders.insert("openrouter", super::openrouter::builder);
        self.builders.insert("gemini_cli", super::gemini_cli::builder);
        self.builders.insert("vertex_api", super::vertex_api::builder);
        self.builders.insert("qwen", super::qwen::builder);
        self.builders.insert("kimi", super::kimi::builder);
        self.builders.insert("claude_oauth", super::claude_oauth::builder);
        self.builders.insert("antigravity", super::antigravity::builder);
    }

    pub fn get_builder(&self, preset_name: &str) -> Option<ClientBuilder> {
        self.builders.get(preset_name).map(|f| f())
    }

    pub fn list(&self) -> Vec<&str> {
        self.builders.keys().copied().collect()
    }
}

pub static REGISTRY: Lazy<PresetRegistry> = Lazy::new(PresetRegistry::new);
