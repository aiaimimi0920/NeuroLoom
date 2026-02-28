use async_trait::async_trait;
use reqwest::Client;

use crate::model::resolver::ModelResolver;
use crate::provider::extension::{ModelInfo, ProviderExtension};

pub struct AlayanewModelResolver {}

impl AlayanewModelResolver {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for AlayanewModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for AlayanewModelResolver {
    fn resolve(&self, model: &str) -> String {
        if model.is_empty() {
            "deepseek-chat".to_string()
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
            "deepseek-chat" => (65536, 8192),
            "deepseek-reasoner" => (65536, 8192),
            _ => (8192, 4096),
        }
    }

    fn intelligence_and_modality(
        &self,
        model: &str,
    ) -> Option<(f32, crate::model::resolver::Modality)> {
        match model {
            "deepseek-chat" | "deepseek-reasoner" => Some((4.0, crate::model::resolver::Modality::Text)), 
            _ => Some((3.5, crate::model::resolver::Modality::Text)),
        }
    }
}

pub struct AlayanewExtension {}

impl AlayanewExtension {
    pub fn new(_base_url: impl Into<String>) -> Self {
        Self {}
    }

    fn fallback_models() -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                id: "deepseek-chat".to_string(),
                description: "DeepSeek Chat V3 (AlayaNew)".to_string(),
            },
            ModelInfo {
                id: "deepseek-reasoner".to_string(),
                description: "DeepSeek Reasoner R1 (AlayaNew)".to_string(),
            },
        ]
    }
}

#[async_trait]
impl ProviderExtension for AlayanewExtension {
    fn id(&self) -> &str {
        "alayanew"
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
