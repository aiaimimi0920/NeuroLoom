use async_trait::async_trait;
use reqwest::Client;

use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use crate::model::{Capability, ModelResolver};
use crate::provider::extension::{ModelInfo, ProviderExtension};

pub struct AiOnlyModelResolver;

impl AiOnlyModelResolver {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for AiOnlyModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for AiOnlyModelResolver {
    fn resolve(&self, model: &str) -> String {
        if model.is_empty() {
            "gpt-4o".to_string()
        } else {
            model.to_string()
        }
    }

    fn has_capability(&self, _model: &str, capability: Capability) -> bool {
        let supported = Capability::CHAT | Capability::STREAMING;
        supported.contains(capability)
    }

    fn max_context(&self, _model: &str) -> usize {
        128_000
    }

    fn context_window_hint(&self, _model: &str) -> (usize, usize) {
        (100_000, 28_000)
    }

    fn intelligence_and_modality(
        &self,
        _model: &str,
    ) -> Option<(f32, crate::model::resolver::Modality)> {
        Some((3.5, crate::model::resolver::Modality::Text))
    }
}

pub struct AiOnlyExtension;

impl AiOnlyExtension {
    pub fn new(_base_url: impl Into<String>) -> Self {
        Self {}
    }

    fn fallback_models() -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                id: "gpt-4o".to_string(),
                description: "OpenAI GPT-4o (AiOnly Proxy)".to_string(),
            },
            ModelInfo {
                id: "claude-3-5-sonnet-20240620".to_string(),
                description: "Claude 3.5 Sonnet (AiOnly Proxy)".to_string(),
            },
            ModelInfo {
                id: "gemini-1.5-pro".to_string(),
                description: "Gemini 1.5 Pro (AiOnly Proxy)".to_string(),
            },
        ]
    }
}

#[async_trait]
impl ProviderExtension for AiOnlyExtension {
    fn id(&self) -> &str {
        "aionly"
    }

    async fn list_models(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(Self::fallback_models())
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        ConcurrencyConfig::new(20)
    }
}
