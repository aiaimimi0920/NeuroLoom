use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

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

pub struct Ph8Extension {
    base_url: String,
}

impl Ph8Extension {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
        }
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

    async fn fetch_remote_models(
        &self,
        http: &Client,
        auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        let request = http.get(format!("{}/models", self.base_url));
        let request = auth.inject(request)?;

        let resp = request.send().await?;
        let status = resp.status();
        if !status.is_success() {
            anyhow::bail!("PH8 /models request failed with status {status}");
        }

        let payload: Value = resp.json().await?;
        let mut models = payload
            .get("data")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| {
                        let id = item.get("id").and_then(Value::as_str)?.trim();
                        if id.is_empty() {
                            return None;
                        }
                        Some(ModelInfo {
                            id: id.to_string(),
                            description: item
                                .get("owned_by")
                                .and_then(Value::as_str)
                                .map(|owner| format!("{id} ({owner})"))
                                .unwrap_or_else(|| id.to_string()),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        models.sort_by(|a, b| a.id.cmp(&b.id));
        models.dedup_by(|a, b| a.id == b.id);
        if models.is_empty() {
            anyhow::bail!("PH8 /models returned empty model list");
        }

        Ok(models)
    }
}

#[async_trait]
impl ProviderExtension for Ph8Extension {
    fn id(&self) -> &str {
        "ph8"
    }

    async fn list_models(
        &self,
        http: &Client,
        auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        match self.fetch_remote_models(http, auth).await {
            Ok(models) => Ok(models),
            Err(_) => Ok(Self::fallback_models()),
        }
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        ConcurrencyConfig::new(20)
    }
}
