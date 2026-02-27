use crate::model::resolver::ModelResolver;
use crate::provider::extension::{ModelInfo, ProviderExtension};

pub struct SophnetModelResolver {}

impl SophnetModelResolver {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for SophnetModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for SophnetModelResolver {
    fn resolve(&self, model: &str) -> String {
        if model.is_empty() {
            "DeepSeek-v3".to_string() 
        } else {
            model.to_string()
        }
    }

    fn has_capability(&self, _model: &str, capability: crate::model::Capability) -> bool {
        let supported = crate::model::Capability::CHAT | crate::model::Capability::STREAMING;
        supported.contains(capability)
    }

    fn max_context(&self, _model: &str) -> usize {
        128_000
    }

    fn context_window_hint(&self, model: &str) -> (usize, usize) {
        match model {
            "DeepSeek-v3" => (32768, 8192),
            _ => (8192, 4096),
        }
    }

    fn intelligence_and_modality(
        &self,
        model: &str,
    ) -> Option<(f32, crate::model::resolver::Modality)> {
        match model {
            "DeepSeek-v3" => Some((3.5, crate::model::resolver::Modality::Text)), 
            _ => Some((3.0, crate::model::resolver::Modality::Text)),
        }
    }
}

use async_trait::async_trait;
use reqwest::Client;

pub struct SophnetExtension {
    base_url: String,
}

impl SophnetExtension {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
        }
    }

    fn fallback_models() -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                id: "DeepSeek-v3".to_string(),
                description: "DeepSeek V3 (SophNet)".to_string(),
            },
            ModelInfo {
                id: "Qwen3-32B".to_string(),
                description: "Qwen 3 32B (SophNet)".to_string(),
            },
        ]
    }
}

#[async_trait]
impl ProviderExtension for SophnetExtension {
    fn id(&self) -> &str {
        "sophnet"
    }

    async fn list_models(
        &self,
        _http: &Client,
        _auth: &mut dyn crate::auth::traits::Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(Self::fallback_models())
    }

    fn concurrency_config(&self) -> crate::concurrency::ConcurrencyConfig {
        crate::concurrency::ConcurrencyConfig::new(2)
    }
}
