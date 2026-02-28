use crate::model::resolver::ModelResolver;
use crate::provider::extension::{ModelInfo, ProviderExtension};
use serde_json::Value;

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
        match model.trim().to_lowercase().as_str() {
            "" => "DeepSeek-v3".to_string(),
            "deepseek-v3" | "deepseek_v3" | "deepseek" | "ds-v3" => "DeepSeek-v3".to_string(),
            "qwen3-32b" | "qwen-3-32b" | "qwen32b" => "Qwen3-32B".to_string(),
            _ => model.to_string(),
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
        match self.resolve(model).as_str() {
            "DeepSeek-v3" => (65_536, 16_384),
            "Qwen3-32B" => (32_768, 8_192),
            _ => (8_192, 4_096),
        }
    }

    fn intelligence_and_modality(
        &self,
        model: &str,
    ) -> Option<(f32, crate::model::resolver::Modality)> {
        match self.resolve(model).as_str() {
            "DeepSeek-v3" => Some((3.7, crate::model::resolver::Modality::Text)),
            "Qwen3-32B" => Some((3.4, crate::model::resolver::Modality::Text)),
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
            base_url: base_url.into().trim_end_matches('/').to_string(),
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

    async fn fetch_remote_models(
        &self,
        http: &Client,
        auth: &mut dyn crate::auth::traits::Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        let request = http.get(format!("{}/models", self.base_url));
        let request = auth.inject(request)?;

        let resp = request.send().await?;
        let status = resp.status();
        if !status.is_success() {
            anyhow::bail!("SophNet /models request failed with status {status}");
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
            anyhow::bail!("SophNet /models returned empty model list");
        }

        Ok(models)
    }
}

#[async_trait]
impl ProviderExtension for SophnetExtension {
    fn id(&self) -> &str {
        "sophnet"
    }

    async fn list_models(
        &self,
        http: &Client,
        auth: &mut dyn crate::auth::traits::Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        match self.fetch_remote_models(http, auth).await {
            Ok(models) => Ok(models),
            Err(_) => Ok(Self::fallback_models()),
        }
    }

    fn concurrency_config(&self) -> crate::concurrency::ConcurrencyConfig {
        crate::concurrency::ConcurrencyConfig::new(20)
    }
}

#[cfg(test)]
mod tests {
    use super::SophnetModelResolver;
    use crate::model::ModelResolver;

    #[test]
    fn resolves_aliases_to_canonical_model_names() {
        let resolver = SophnetModelResolver::new();
        assert_eq!(resolver.resolve(""), "DeepSeek-v3");
        assert_eq!(resolver.resolve("deepseek"), "DeepSeek-v3");
        assert_eq!(resolver.resolve("qwen-3-32b"), "Qwen3-32B");
    }

    #[test]
    fn provides_model_specific_context_hints() {
        let resolver = SophnetModelResolver::new();
        assert_eq!(
            resolver.context_window_hint("DeepSeek-v3"),
            (65_536, 16_384)
        );
        assert_eq!(resolver.context_window_hint("qwen3-32b"), (32_768, 8_192));
    }
}
