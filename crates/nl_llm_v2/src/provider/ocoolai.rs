use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use crate::model::{Capability, ModelResolver};
use crate::provider::extension::{ModelInfo, ProviderExtension};

pub struct OcoolAiModelResolver;

impl OcoolAiModelResolver {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for OcoolAiModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for OcoolAiModelResolver {
    fn resolve(&self, model: &str) -> String {
        if model.is_empty() {
            "gpt-4o-mini".to_string()
        } else {
            model.to_string()
        }
    }

    fn has_capability(&self, _model: &str, capability: Capability) -> bool {
        let supported = Capability::CHAT | Capability::STREAMING | Capability::TOOLS;
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

#[derive(Debug, Deserialize)]
struct OcoolAiModelsResponse {
    #[serde(default)]
    data: Vec<OcoolAiModelItem>,
}

#[derive(Debug, Deserialize)]
struct OcoolAiModelItem {
    id: String,
}

pub struct OcoolAiExtension {
    base_url: String,
}

impl OcoolAiExtension {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
        }
    }

    fn models_url(&self) -> String {
        format!("{}/models", self.base_url)
    }
}

#[async_trait]
impl ProviderExtension for OcoolAiExtension {
    fn id(&self) -> &str {
        "ocoolai"
    }

    async fn list_models(
        &self,
        http: &Client,
        auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        let req = auth.inject(http.get(self.models_url()))?;
        let resp = req.send().await?;
        if !resp.status().is_success() {
            return Ok(vec![ModelInfo {
                id: "gpt-4o-mini".to_string(),
                description: "Fallback model".to_string(),
            }]);
        }

        let payload: OcoolAiModelsResponse = resp
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("ocoolai /models 解析失败: {}", e))?;

        Ok(payload
            .data
            .into_iter()
            .map(|m| ModelInfo {
                id: m.id,
                description: String::new(),
            })
            .collect())
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        ConcurrencyConfig::new(20)
    }
}
