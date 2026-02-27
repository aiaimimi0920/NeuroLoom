use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

use crate::auth::traits::Authenticator;
use crate::concurrency::ConcurrencyConfig;
use crate::model::{Capability, ModelResolver};
use crate::provider::extension::{ModelInfo, ProviderExtension};

pub struct PoeModelResolver;

impl PoeModelResolver {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for PoeModelResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelResolver for PoeModelResolver {
    fn resolve(&self, model: &str) -> String {
        let normalized = model.trim();
        if normalized.is_empty() {
            return "gpt-4o-mini".to_string();
        }

        // 兼容常见的大小写/别名写法，减少用户输入差异导致的 404。
        let lower = normalized.to_ascii_lowercase();
        match lower.as_str() {
            "gpt-4o" => "gpt-4o".to_string(),
            "gpt-4o-mini" => "gpt-4o-mini".to_string(),
            "claude-3.5-sonnet" => "claude-3-5-sonnet".to_string(),
            _ => normalized.to_string(),
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

#[derive(Debug, Deserialize)]
struct PoeModelsResponse {
    #[serde(default)]
    data: Vec<PoeModelItem>,
}

#[derive(Debug, Deserialize)]
struct PoeModelItem {
    id: String,
}

pub struct PoeExtension {
    base_url: String,
}

impl PoeExtension {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
        }
    }

    fn models_url(&self) -> String {
        format!("{}/models", self.base_url)
    }

fn fallback_models() -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                id: "gpt-4o-mini".to_string(),
                description: "Poe fallback model".to_string(),
            },
            ModelInfo {
                id: "claude-3-5-sonnet".to_string(),
                description: "Poe fallback model".to_string(),
            },
        ]
    }
}

#[async_trait]
impl ProviderExtension for PoeExtension {
    fn id(&self) -> &str {
        "poe"
    }

    async fn list_models(
        &self,
        http: &Client,
        auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        let req = auth.inject(http.get(self.models_url()))?;

        match req.send().await {
            Ok(resp) if resp.status().is_success() => {
                let payload: PoeModelsResponse = resp
                    .json()
                    .await
                    .map_err(|e| anyhow::anyhow!("poe /models 解析失败: {}", e))?;
                if payload.data.is_empty() {
                    Ok(Self::fallback_models())
                } else {
                    Ok(payload
                        .data
                        .into_iter()
                        .map(|m| ModelInfo {
                            id: m.id,
                            description: String::new(),
                        })
                        .collect())
                }
            }
            Ok(_) | Err(_) => Ok(Self::fallback_models()),
        }
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        ConcurrencyConfig::new(20)
    }
}
