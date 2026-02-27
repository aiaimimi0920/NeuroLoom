use async_trait::async_trait;
use reqwest::Client;

use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use crate::model::{Capability, ModelResolver};
use crate::provider::extension::{ModelInfo, ProviderExtension};

pub struct Ph8ModelResolver;

impl Ph8ModelResolver {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for Ph8ModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for Ph8ModelResolver {
    fn resolve(&self, model: &str) -> String {
        if model.is_empty() {
            "qwen-max".to_string()
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
        model: &str,
    ) -> Option<(f32, crate::model::resolver::Modality)> {
        match model {
            "qwen-max" => Some((4.0, crate::model::resolver::Modality::Text)),
            "qwen-plus" => Some((3.5, crate::model::resolver::Modality::Text)),
            _ => Some((3.5, crate::model::resolver::Modality::Text)),
        }
    }
}

pub struct Ph8Extension;

impl Ph8Extension {
    pub fn new(_base_url: impl Into<String>) -> Self {
        Self {}
    }

    fn fallback_models() -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                id: "qwen-max".to_string(),
                description: "Qwen Max (PH8)".to_string(),
            },
            ModelInfo {
                id: "qwen-plus".to_string(),
                description: "Qwen Plus (PH8)".to_string(),
            },
            ModelInfo {
                id: "claude-opus-4.6".to_string(),
                description: "Claude Opus 4.6 (PH8)".to_string(),
            },
        ]
    }
}

#[async_trait]
impl ProviderExtension for Ph8Extension {
    fn id(&self) -> &str {
        "ph8"
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
